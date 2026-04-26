# Investigation — ha-tool ↔ remote-shell integration audit

## Bug summary

The remote-shell extension was updated to format all tool outputs as
**human-readable text** (e.g. `"Exit code: 0\n--- stdout ---\n…"`,
`"Connected successfully.\nSession ID: …"`), but `./tools-src/ha-tool/src/shell.rs`
still parses every response as **raw JSON** with fields like
`exit_code`, `stdout`, `stderr`, and `session_id`.

As a result, **every shell-backed code path in `ha-tool` is broken**:

- `shell_read_file`, `shell_write_file`, `shell_tail_file`, and `ha_cli` always
  fail with `"invalid shell response: …"` because `parse_exec_output`
  cannot find `exit_code` in a plain-text string.
- Opening a new SSH session through ha-tool always fails with
  `"remote-shell connect returned invalid JSON"`.
- For `check_config`, `get_error_log`, `restart_ha` (the auto-shell-preferring
  REST actions), `try_shell` swallows the parse error as a “shell unavailable”
  signal and silently falls back to REST. The user’s SSH config is ignored.
- For `restart_ha` (which uses `try_shell_strict`), the parse error
  propagates instead of falling back, producing a confusing error
  even though the gateway succeeded.

There are also several secondary integration / quality issues uncovered
during the audit (probe action, gateway-port propagation, magic numbers,
unreachable size cap).

## Root cause analysis

The two repos share a contract that has drifted:

| Action          | remote-shell `output`                          | ha-tool expects                                        |
|-----------------|------------------------------------------------|--------------------------------------------------------|
| `connect`       | `"Connected successfully.\nSession ID: <id>\n<msg>\n\n…"` | JSON `{"session_id": "<id>", …}`                        |
| `execute`       | `"Exit code: <n>\n--- stdout ---\n…--- stderr ---\n…"`     | JSON `{"exit_code": n, "stdout": "…", "stderr": "…"}`   |
| `list_sessions` | `"Active sessions (N):\n…"` or `"No active sessions."`     | Any Ok (used only as a probe)                          |
| `disconnect`    | `"Session '<id>' disconnected successfully."`              | (not invoked)                                          |
| `health`        | `"Gateway is reachable at …"`                              | (not invoked — but should be the probe)                |

Two helpers in `./tools-src/ha-tool/src/shell.rs` rely on the obsolete
JSON shape:

1. `ensure_session` (lines 152–159) — `serde_json::from_str(&resp)` on text.
2. `parse_exec_output` (lines 186–201) — same on every `shell_exec` result.

Additional integration issues:

3. `is_shell_available` (lines 49–53) probes via `list_sessions` (works,
   but heavier than the new `health` action) and never propagates
   `gateway_port`. With a non-default gateway port, the probe always
   fails — every shell-aware call silently falls back to REST.
4. `write_file` advertises `MAX_FILE_WRITE_LEN = 1 MiB`, but the bytes are
   base64-encoded and stuffed into a shell command bounded by
   `MAX_COMMAND_LEN = 64 KiB`. The reachable cap is ~48 KiB, the rest is
   dead validation.
5. `is_shell_available()` makes a network round-trip on every shell-aware
   call. With multiple shell ops per heartbeat tick, this adds latency and
   one extra gateway request per call. Should be cached per `execute`
   invocation.
6. `try_shell` swallows shell errors (including auth failures and unknown
   sessions) into a `Warn` log and falls back to REST. From the user’s POV
   the SSH path “just doesn’t work” — they get a REST result with no
   indication their credentials were wrong. For `check_config` and
   `get_error_log` this is acceptable; for explicitly shell-only ops
   (write_file, ha_cli, …) the error path already propagates correctly.

Plus a few low-severity items in the supporting material:

7. `./skills/SKILL.md` doesn’t mention the new `health`-style probe
   semantics or that `shell_status` already maps to the gateway probe.
8. `./skills/SKILL.md` claims `shell_exec` is “root-equivalent” which is
   true on Home Assistant OS / Supervised but **not** on a plain Linux
   install where the SSH user may be unprivileged. Phrase as
   “privileges of the SSH user (often root on HA OS)”.
9. `./heartbeat/HEARTBEAT.md` references `heartbeat/ha-last-log.md` and
   `heartbeat/ha-latest.md` paths but the install script doesn’t create
   the `heartbeat/` directory under `~/.ironclaw`. The agent must `mkdir`
   on first run, but this isn’t spelled out.

## Affected components

- `./tools-src/ha-tool/src/shell.rs` (primary)
  - `ensure_session` — JSON parse on text response.
  - `parse_exec_output` — JSON parse on text response.
  - `is_shell_available` — wrong probe action, missing gateway-port.
  - `write_file` — unreachable size cap (cosmetic but misleading).
- `./tools-src/ha-tool/src/api.rs` — indirectly: every `try_shell(...)` /
  `try_shell_strict(...)` site silently degrades to REST.
- `./skills/SKILL.md` — minor wording fixes (`shell_status` description,
  privilege framing).
- `./heartbeat/HEARTBEAT.md` — minor: clarify directory creation.
- No changes needed in `./wit/tool.wit`, `./tools-src/ha-tool/src/lib.rs`,
  `./tools-src/ha-tool/src/types.rs`, capabilities JSON, or scripts.

## Proposed solution

### A. Fix the wire format (critical)

Add a small helper in `./tools-src/ha-tool/src/shell.rs` that extracts the
fields ha-tool needs from the **human-formatted** strings remote-shell now
returns. Two parsers, both regex-free and string-based:

1. `parse_connect_response(&str) -> Result<String, String>` — scan for the
   line beginning `"Session ID: "` and return the trimmed remainder.
2. `parse_exec_output(&str) -> Result<(i32, String, String), String>` —
   scan for `"Exit code: "` (numeric → `i32`, `unknown …` → `-1`),
   then split off the `--- stdout ---` and `--- stderr ---` blocks
   (each block is everything between its header and the next header /
   end of string). Both blocks may be absent (the "(no output)" form).

Update call sites:

- `ensure_session` — replace `serde_json::from_str` with
  `parse_connect_response`.
- `read_file`, `write_file`, `tail_file` — keep using `parse_exec_output`
  (now text-based).
- `ha_cli` returns `shell_exec`’s raw text directly — fine, the agent
  reads the formatted string.

### B. Use `health` as the probe action

Replace the `list_sessions` probe with `health` and forward
`gateway_port`:

```rust
fn is_shell_available(gateway_port: Option<u16>) -> bool {
    let mut body = serde_json::json!({"action": "health"});
    if let Some(p) = gateway_port { body["gateway_port"] = p.into(); }
    host::tool_invoke(REMOTE_SHELL_ALIAS, &body.to_string()).is_ok()
}
```

Thread `gateway_port` through `try_shell` and `try_shell_strict` so the
probe targets the same gateway the actual command will hit.

### C. Cache the probe within a single tool invocation

Run the probe at most once per `execute_inner` call. Easiest: pass an
`Option<bool>` flag down, or memoize via a `thread_local!` cell that
`HaTool::execute` clears at entry. Picking the flag-based approach to
keep behaviour explicit.

### D. Fix the dead-code / magic-number cap on `write_file`

Either:
- lower `MAX_FILE_WRITE_LEN` to a value actually reachable through
  `MAX_COMMAND_LEN` (≈ `(MAX_COMMAND_LEN - 64) * 3 / 4` ≈ 49 000), **or**
- chunk the write across multiple `execute` calls.

The simplest correct fix is to lower the cap and document it in the
error message. Chunking adds non-trivial atomicity concerns (we’d need
a temp file + `mv`) and is outside the current scope.

### E. Skill / heartbeat copy fixes

- `./skills/SKILL.md`:
  - Reword `shell_exec` privileges: “runs with the privileges of the SSH
    user (typically root on HA OS / Supervised)”.
  - Add `shell_status` returns a JSON object including
    `remote_shell_available`; agents should check this before opting
    into shell-aware actions on a fresh session.
- `./heartbeat/HEARTBEAT.md`: note that `heartbeat/ha-last-log.md` and
  `heartbeat/ha-latest.md` are workspace-relative paths; the agent must
  create the directory on first tick.

### F. Regression tests

Add unit tests in `./tools-src/ha-tool/src/shell.rs::tests` for:

- `parse_connect_response` happy path + missing line.
- `parse_exec_output` for each shape: stdout-only, stderr-only,
  both, "(no output)", `Exit code: unknown …`.
- Round-trip: feed the literal strings remote-shell produces (copied
  verbatim from `format_connect_response` / `format_execute_response`)
  and assert the parsed values.

These tests fail before the fix (current code calls
`serde_json::from_str` on text) and pass after.

## Edge cases & side effects

- remote-shell may evolve again — the parser should accept extra trailing
  lines (e.g. the “Use this session_id…” suffix on connect) without
  breaking. We ignore everything we don’t recognise.
- A successful `Exit code: 0` with empty output yields the “(no output)”
  branch in remote-shell. `parse_exec_output` must treat that as
  `(0, "", "")`, not an error.
- `Exit code: unknown (command may have timed out)` → map to `-1` so
  call sites observe a non-zero exit and surface the stderr (typically
  empty) along with the timeout indicator preserved in the raw body
  used by `ha_cli`.
- For `read_file`, the file content is now embedded inside a
  text-formatted block. Currently `read_file` returns
  `{"path": …, "content": stdout}`. We keep that envelope; `stdout`
  comes from the text parser.
- For `write_file`, the lowered cap is a behaviour change — but the
  current advertised cap is unreachable, so users hitting >49 KiB
  already see opaque “command too long” errors. The new error message
  will be clearer.
- The `health`-based probe means callers using a non-default
  `gateway_port` will now correctly detect availability. No regression.
- All existing `cargo test` cases for `api.rs` continue to pass — the
  fix is contained to `shell.rs`.

## Implementation notes

Applied in this session:

- **A. Wire-format parsers** — `./tools-src/ha-tool/src/shell.rs`:
  - Added `parse_connect_response(&str)` extracting the session id from the
    `Session ID: <id>` line; tolerates extra surrounding lines.
  - Replaced `parse_exec_output` with a text-based parser that reads
    `Exit code: <n|unknown ...>` and slices `--- stdout ---` / `--- stderr ---`
    blocks. Strips exactly one separator newline between blocks so a
    round-trip through `format_execute_response` is byte-exact.
  - `ensure_session` now calls `parse_connect_response` instead of
    `serde_json::from_str`.
- **B. `health` probe + gateway-port propagation** — `is_shell_available`
  now takes `Option<u16>` and invokes `{"action": "health", "gateway_port": …}`.
  `try_shell` and `try_shell_strict` forward `cfg.gateway_port`. No call-site
  change needed in `./tools-src/ha-tool/src/api.rs`.
- **C. Probe caching** — Confirmed every `execute_inner` invocation issues
  at most one `try_shell` (one shell HaAction maps to one match arm), so the
  probe already runs at most once per call. No caching layer added (would be
  dead code in the current call graph).
- **D. Reachable write cap** — Lowered `MAX_FILE_WRITE_LEN` from 1 MiB to
  32 KiB. Added `test_max_file_write_len_fits_in_command_budget` that asserts
  a worst-case (`MAX_PATH_LEN` path + skeleton + base64 payload) command
  stays under `MAX_COMMAND_LEN`. Error message now reports the actual size
  and recommends chunking.
- **E. Doc fixes** — `./skills/SKILL.md`: reworded `shell_exec` privileges
  to mention "privileges of the SSH user (typically root on HA OS /
  Supervised)", documented the new 32 KiB `shell_write_file` cap, and noted
  that `shell_status` returns `remote_shell_available`.
  `./heartbeat/HEARTBEAT.md`: clarified that the workspace-relative
  `heartbeat/` directory must be created on first tick.

## Review follow-ups (round 2)

Addressed during code-review pass:

- **Strict body validation in `parse_exec_output`** — When the body is
  non-empty, isn't `(no output)`, and contains no recognisable
  `--- stdout ---` / `--- stderr ---` marker, return an `Err` with the
  raw body instead of silently yielding `("", "")`. Prevents `read_file`
  / `tail_file` from returning empty content as a successful result if
  the wire format ever drifts again.
- **`shell_status` honours `gateway_port`** — Added `gateway_port:
  Option<u16>` to the `ShellStatus` action variant; `lib.rs` forwards it
  to `shell::shell_status`, which forwards it to `is_shell_available`.
  Avoids false-negative availability reports when the remote-shell
  gateway runs on a non-default port.
- **`ha_cli` doc comment** — Added a doc comment explaining the deliberate
  inconsistency: `ha_cli` returns `shell_exec`'s raw human-formatted text
  verbatim (rather than a JSON envelope) so agents can surface the full
  output, including timeout indicators and stderr, to the user.
- **Test alignment with canonical wire format** — Renamed the dual-stream
  test pair: `test_parse_exec_output_both_streams` now uses the
  no-trailing-newline form (single `\n` separator) that the formatter
  produces for stdout content without a trailing `\n`, and
  `test_parse_exec_output_both_streams_trailing_newline` covers the
  trailing-newline form (two `\n` chars before the next marker). Both
  exercise output the remote-shell extension actually emits.
- **New regression test** — `test_parse_exec_output_unknown_body_is_err`
  asserts that footer-only / unknown-format bodies surface as `Err`.

## Test results

`cargo test --lib` — **28 passed, 0 failed**. New regression tests (all
fail against the pre-fix JSON-based implementation, pass after):

- `test_parse_connect_response_happy_path`
- `test_parse_connect_response_missing_line`
- `test_parse_connect_response_empty_id`
- `test_parse_exec_output_no_output`
- `test_parse_exec_output_stdout_only`
- `test_parse_exec_output_stderr_only`
- `test_parse_exec_output_both_streams`
- `test_parse_exec_output_unknown_exit_code`
- `test_parse_exec_output_invalid_header`
- `test_parse_exec_output_roundtrip_format` (mirrors the remote-shell
  formatter to detect future drift)
- `test_max_file_write_len_fits_in_command_budget`

`cargo build --target wasm32-wasip2 --release` — succeeds.

## Out of scope

- Adding a `disconnect` action to ha-tool (sessions already TTL-out on
  the gateway).
- Adding a `health` passthrough action to ha-tool (`shell_status`
  already covers this).
- Refactoring `try_shell` / `try_shell_strict` semantics (current split
  is intentional — destructive ops shouldn’t fall back silently).
