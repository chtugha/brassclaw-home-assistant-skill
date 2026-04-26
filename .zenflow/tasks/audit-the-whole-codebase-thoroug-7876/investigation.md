# Investigation — Full codebase audit

Scope: every file under `./tools-src/ha-tool/`, `./wit/`, `./scripts/`,
`./skills/`, and `./heartbeat/`. Goal: enumerate bugs, stubs, simulations,
dead code, magic numbers, security issues, and token-cost / agent-ergonomics
weaknesses, then propose concrete fixes.

The previous audit (`./.zenflow/tasks/https-github-com-chtugha-ironcla-2669/investigation.md`)
already shipped fixes for the wire-format mismatch with the remote-shell
extension and the magic-number cleanup in `./tools-src/ha-tool/src/shell.rs`.
Those are confirmed in the current source — this audit focuses on what
remains.

---

## Findings

Each finding is tagged `[BUG]`, `[SEC]`, `[DEAD]`, `[MAGIC]`, `[STUB]`,
`[TOKEN]`, `[ERGO]`, or `[DOC]` and ranked by severity (P1 > P2 > P3).

### F1 [BUG] [P2] `get_states.total` reports filtered count, not pre-filter total
File: `./tools-src/ha-tool/src/api.rs:218-256`
The `StatesResponse.total` field is documented (by name) as the unfiltered
size, but the code sets `total = filtered.len()` after `domain_filter` was
applied. Callers asking "how many entities does HA have?" get the
post-filter count.
Fix: capture `let total_unfiltered = all.len()` before filtering, expose
`matched` (filtered count) and `total` (unfiltered) separately. Or rename
the existing field to `matched` and add a separate `total` field.

### F2 [BUG] [P2] `get_history` rejects all `hours_back` values when `start_time` is supplied
File: `./tools-src/ha-tool/src/api.rs:331-344`
When the caller supplies `start_time`, `hours_back` is silently ignored
and never validated. That is the documented intent, but the API also
silently accepts garbage `hours_back` values. More importantly, `end_time`
is not exposed at all, so callers can never request a *bounded window* —
they always get "from `start_time` until now", which can return the full
history (large token cost).
Fix: add optional `end_time: Option<String>` parameter; pass it through
as `?end_time=…` per the HA REST contract.

### F3 [BUG] [P3] `get_logbook` lacks `start_time` / `end_time` parameters
File: `./tools-src/ha-tool/src/api.rs:346-359`
Only `hours_back` is exposed. HA's `/api/logbook/{ts}?end_time=…`
accepts an explicit end. Same token-bloat consequence as F2.
Fix: add optional `start_time` (replaces `hours_back` when present) and
`end_time`.

### F4 [BUG] [P2] `render_template` has no result-size cap
File: `./tools-src/ha-tool/src/api.rs:321-329`
A Jinja2 template like `{{ states | tojson }}` can return megabytes,
which the agent will dump into context. There is no truncation or
warning in the response.
Fix: add `max_chars: Option<u32>` (default `8192`). When the rendered
output exceeds the cap, truncate and append a `…[truncated, N more bytes]`
marker so the agent knows to ask for more / refine.

### F5 [SEC] [P3] `validate_ha_url` accepts URLs with embedded `@` (userinfo)
File: `./tools-src/ha-tool/src/api.rs:35-70`
`validate_ha_url("http://attacker.com@192.168.1.1/")` would currently
parse the host as `attacker.com@192.168.1.1`, fail the private-IP shape
check, and be rejected — *good*. But `http://192.168.1.1@evil.com/` is
also rejected (host = `192.168.1.1@evil.com`, which is not a recognized
private form). However: `http://192.168.1.1#@evil.com` is rejected too.
The validator is currently safe because `@` and `#` and friends are not
in the allowed-host suffix list. The defense is implicit, not explicit.
Fix: explicitly reject any URL whose host part contains `@`, `?`, or `#`
before the suffix check, with a clear error message. Defense-in-depth.

### F6 [SEC] [P3] `ha_url` is *re-validated on every HTTP call* but never *normalized*
File: `./tools-src/ha-tool/src/api.rs:180-208`
`validate_ha_url` checks the lowercased form, then `normalize_url` only
trims a trailing `/`. `http://HA:8123/api/` and `http://ha:8123/API/` are
not consolidated. Minor, but means rate-limit accounting on the host
side may treat them as different endpoints.
Fix: lowercase the scheme + host (preserve case-sensitive path) before
constructing the request URL. Low priority — IronClaw rate-limits per
tool, not per URL, so this is purely cosmetic for now.

### F7 [MAGIC] [P3] `iso_timestamp_hours_ago` magic numbers `60`
File: `./tools-src/ha-tool/src/api.rs:166-178`
`SECONDS_PER_HOUR` and `SECONDS_PER_DAY` are constants, but the inner
`/ 60` and `% 60` for minutes/seconds are bare. `days_to_ymd` (line 549)
is full of magic numbers (`719468`, `146097`, `146096`, `1460`, `36524`,
`365`, `153`, `5`, …) — these are the well-known
[Howard Hinnant date algorithm](https://howardhinnant.github.io/date_algorithms.html)
constants and *should not* be turned into named constants (they only
make sense as a group), but they should carry a `// Hinnant 2013, civil_from_days` comment so future readers don't try to "fix" them.
Fix: extract `SECONDS_PER_MINUTE = 60`; add a one-line algorithm
attribution comment above `days_to_ymd`.

### F8 [DEAD] [P3] `MAX_HOURS_BACK = 8760` is enforced for `get_history` only when `start_time` is `None` and for `get_logbook` always
File: `./tools-src/ha-tool/src/api.rs:331-359`
Asymmetric enforcement is deliberate but easy to misread; document it.

### F9 [TOKEN] [P1] `get_states` response contains every entity attribute (state, attributes object, last_changed, last_updated, context)
File: `./tools-src/ha-tool/src/api.rs:218-256`
A medium HA install (~200 entities) produces ~80 KB of JSON, mostly
attributes that the agent doesn't need for discovery. This is **the
single largest token sink** in the tool.
Fix: add `compact: Option<bool>` (default `false`). When `true`, project
each entity to `{entity_id, state, last_changed?}` only, dropping the
full `attributes` map. Document it in the capability summary so the
agent reaches for it during discovery and falls back to `get_state` for
attribute access.

### F10 [TOKEN] [P2] No multi-domain `domain_filter`
File: `./tools-src/ha-tool/src/api.rs:218-235`
Agents commonly want "give me lights AND switches AND sensors" in one
call to avoid 3× the round-trip + 3× the wrapper JSON. Currently they
must fetch each domain separately or fetch everything.
Fix: accept `domain_filter` as either a string or an array of strings
(serde untagged enum or `Vec<String>`) and union the matches.

### F11 [TOKEN] [P2] `description()` and `schema()` are recomputed on every host call
File: `./tools-src/ha-tool/src/lib.rs:28-41`
`schemars::schema_for!(types::HaAction)` produces a ~6 KB JSON object on
each call. While the host caches them externally in practice, the WASM
side could memoize via `OnceLock` to avoid wasm32 codegen re-running
the derive at runtime.
Fix: wrap the result in `static SCHEMA: OnceLock<String> = …`.
Marginal (host probably only calls `schema()` once per registration),
but cheap.

### F12 [ERGO] [P2] `description()` is dense prose without machine-readable hints
File: `./tools-src/ha-tool/src/lib.rs:33-41`
The string blends "what it does" with "how to invoke" prose. The
capability JSON already has structured `discovery_summary`, so the
description duplicates that information in tokens the agent re-reads
each turn.
Fix: tighten to one sentence ("Control Home Assistant via REST API:
states, services, automations, scripts, scenes, MQTT, Modbus, templates,
history, logs, and reloads. Requires `ha_url` on every call. See
`discovery_summary` for parameters."). Saves ~80 tokens per turn the
description is in scope.

### F13 [ERGO] [P2] `HaAction::ListAutomations` / `ListScripts` / `ListScenes` are aliases of `get_states` with a domain_filter
File: `./tools-src/ha-tool/src/lib.rs:87-100`
They're convenient but bloat the schema (3 extra enum variants) and the
agent's mental model. Either keep them and document their equivalence,
or remove and let the agent use `get_states {domain_filter: …}`. The
schema bloat costs ~150 tokens whenever the schema is in context.
Fix: keep them (they're discoverable shortcuts) but make them honor the
same `compact`/`max_items` parameters once F9 lands.

### F14 [ERGO] [P3] `set_state` does not enforce `attributes` is an object
File: `./tools-src/ha-tool/src/api.rs:263-275`
`attributes: Option<serde_json::Value>` accepts arrays, numbers, etc.,
which HA will reject with a 400. The agent gets a confusing
upstream error.
Fix: on receipt, return early if `attrs.is_object()` is false.

### F15 [ERGO] [P3] No `delete_state` action
HA exposes `DELETE /api/states/{entity_id}` to remove a manually-created
state. Useful for cleaning up after `set_state` experiments. Currently
missing.
Fix: add `HaAction::DeleteState { ha_url, entity_id }`.

### F16 [ERGO] [P3] `render_template` doesn't accept template `variables`
File: `./tools-src/ha-tool/src/api.rs:321-329`
HA's REST endpoint accepts `{"template": "...", "variables": {...}}`.
Without variables the agent must inline state lookups, which is verbose
and slower.
Fix: add optional `variables: Option<serde_json::Value>`.

### F17 [BUG] [P3] `get_states` over-trims to `MAX_STATES = 500` even when caller asks for more
File: `./tools-src/ha-tool/src/api.rs:238-247`
`Some(n) => (n as usize).min(MAX_STATES)` silently caps. There is no
warning that the cap was hit; the agent sees `truncated: true` but no
indication the *user-supplied* cap was lower than what they asked for.
Fix: distinguish `truncated_by_max_items` vs `truncated_by_hard_cap`
in the response, or return an error if `max_items > MAX_STATES`.
Alternative: raise `MAX_STATES` to e.g. 5000 — it's a token concern only,
not a memory one (we already hold the full vec in memory).

### F18 [SEC] [P3] `shell_exec` is fully un-sandboxed and accepts any command up to 64 KiB
File: `./tools-src/ha-tool/src/shell.rs:183-205`
This is documented in `./skills/SKILL.md` ("intentionally unrestricted"),
and the gateway also enforces auth, but the local-side input validation
is minimal (length + non-empty). Add a hard refusal for the obvious
foot-guns: NUL bytes (already passes through), and optionally a
configurable deny-list for `rm -rf /`, `mkfs`, `dd of=/dev/`, etc. —
but this risks paternalism. Recommended: keep the current behavior,
but add a NUL-byte rejection (matches `validate_path`).
Fix: in `shell_exec`, reject commands containing `\0` with a clear error.

### F19 [BUG] [P3] `parse_exec_output` treats ANY body starting with neither marker as an error — including responses that contain `(no output)` mid-text
File: `./tools-src/ha-tool/src/shell.rs:247-258`
Defensive, and matches the contract from the remote-shell extension's
`format_execute_response`. But: if the gateway one day adds a footer
line (e.g. `Warning: command exceeded soft limit`), every shell call
breaks. Mitigation: tolerate trailing lines after the recognized blocks.
Fix: after extracting stdout/stderr, ignore any unparsed trailing text
unless *both* markers are missing AND the body is non-empty.

### F20 [TOKEN] [P2] `is_shell_available` makes a network round-trip on every shell-aware action
File: `./tools-src/ha-tool/src/shell.rs:51-59, 74-90`
For a heartbeat tick that runs `check_config` + `get_error_log` +
`shell_read_file`, that's three identical probe RPCs **only if** they
are issued from three separate `execute()` calls (which is the common
case — the agent issues one tool call per logical action). Within a
single `execute()`, only one shell-aware path is invoked, so caching
inside `execute` would be dead code today.
Decision: **defer** — caching only pays off if a future caller batches
multiple shell-aware actions into a single `execute()`. Re-evaluate
when/if a batch action is added. If implemented, use
`thread_local! { static AVAILABLE: RefCell<HashMap<u16, bool>> }`
keyed by `gateway_port`, populated lazily on first probe per request.

### F21 [ERGO] [P2] Shell-fallback errors (try_shell) are logged at `Warn` and silently routed to REST
File: `./tools-src/ha-tool/src/shell.rs:74-90`
Acceptable for `check_config` and `get_error_log`, but the user has no
visibility into "your SSH credentials were wrong, here's a REST result
instead." For an agent debugging an issue, this is opaque.
Fix: include a `_shell_fallback: {tried: true, reason: "<msg>"}` field
in the response wrapper for shell-aware REST actions, OR keep the
current behavior but document it loudly in `./skills/SKILL.md`.

### F22 [DOC] [P2] `./skills/SKILL.md` does not mention `shell_status` in its read-only checklist
File: `./skills/SKILL.md:649`
`shell_status` is mentioned in the actions list but the heartbeat /
discovery flow doesn't tell the agent to call it once on session start.
Without this, the agent can't tell whether SSH-aware actions will work.
Fix: add a one-liner to the "Workflow Tips" section: *"On a fresh
session that intends to use shell-aware actions, call `shell_status`
once and cache the result."*

### F23 [DOC] [P2] `./heartbeat/HEARTBEAT.md` references `heartbeat/ha-last-log.md` and `heartbeat/ha-latest.md` paths but install.sh never creates the directory
File: `./scripts/install.sh:30-46`
The agent has to `mkdir` on the first tick. Acceptable, but mention it
in HEARTBEAT.md so the agent knows.
Fix: HEARTBEAT.md already says "create it on the first tick if it does
not yet exist" — *good*. No change needed; verified.

### F24 [DOC] [P3] `README.md` example for `is_private_172` host range says `172.16-31.*` but the implementation uses inclusive `16..=31` — consistent.
No fix.

### F25 [TOKEN] [P3] `StatesResponse` fields use `entities` (plural noun) but every other response is raw HA JSON
This breaks the agent's mental model: most responses are raw HA, but
`get_states` is wrapped. The wrapper costs ~30 tokens per call.
Fix: keep the wrapper (the `count`/`total`/`truncated` metadata is too
useful to drop), but document the divergence prominently.

### F26 [DEAD] [P3] `Cargo.toml [workspace]` block is empty
File: `./tools-src/ha-tool/Cargo.toml:24`
`[workspace]` with no members forces this crate to be its own workspace,
which is intentional (avoids being absorbed by a parent workspace), but
the empty block looks like an oversight. Add a comment.
Fix: `[workspace]\n# Standalone crate — do not let parent workspaces absorb it.`

### F27 [TOKEN] [P3] `description()` strings inside `JsonSchema` derive are auto-generated and verbose
File: `./tools-src/ha-tool/src/types.rs`
Each `serde(default)` field gets a default schema description. The
generated schema is large (~6 KB). Trimming is not worthwhile unless we
pre-build a hand-written schema, which is high effort.
Fix: defer.

### F28 [BUG] [P3] `validate_iso_prefix` accepts `2024-99-99T...` (no month/day range check)
File: `./tools-src/ha-tool/src/api.rs:151-164`
The validator is structural-only. HA will reject the malformed date,
so it's not exploitable, but the error message is noisier than needed.
Fix: range-check month (`01..=12`) and day (`01..=31`) — keep it cheap,
no need for full leap-year math.

### F29 [SEC] [P3] `validate_ha_url` lowercases the entire URL for the host check, but uses the *original* `ha_url` when constructing the request URL
File: `./tools-src/ha-tool/src/api.rs:35-70, 180-208`
A scheme like `HTTP://192.168.1.1` would pass the lowercased check and
then be passed verbatim to `host::http_request`. Most clients tolerate
mixed-case schemes, but it's worth normalizing.
Fix: pass the lowercased scheme through to `normalize_url`.

### F30 [ERGO] [P3] `description()` doesn't mention `ssh` parameter exists for shell-aware actions
File: `./tools-src/ha-tool/src/lib.rs:33-41`
The agent needs to discover from the schema that `check_config`,
`get_error_log`, and `restart_ha` accept `ssh`. The capability JSON
documents it, but the description doesn't.
Fix: short mention in description: "Optional `ssh` on `check_config`,
`get_error_log`, `restart_ha` enables shell-backed mode via the
remote-shell extension."

---

## Affected components

- `./tools-src/ha-tool/src/api.rs` — F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F14, F15, F16, F17, F28, F29
- `./tools-src/ha-tool/src/shell.rs` — F18, F19, F20, F21
- `./tools-src/ha-tool/src/types.rs` — F2, F3, F4, F9, F10, F14, F15, F16, F17, F27
- `./tools-src/ha-tool/src/lib.rs` — F11, F12, F13, F15, F16, F30
- `./tools-src/ha-tool/Cargo.toml` — F26
- `./skills/SKILL.md` — F12, F21, F22
- `./heartbeat/HEARTBEAT.md` — verified, no change

---

## Proposed solution (consolidated)

Apply the following in a single PR, grouped by file. Tests added/updated
inline.

### `./tools-src/ha-tool/src/api.rs`

1. **F1**: track `total_unfiltered` separately; rename `StatesResponse.total` → `matched`, add new `total` field.
2. **F2 / F3**: add `start_time` / `end_time` parameters to `get_history` and `get_logbook`; fall back to `hours_back` only when both are `None`.
3. **F4**: introduce `MAX_TEMPLATE_OUT_BYTES = 16_384`, add `max_chars` parameter to `render_template`, truncate + append marker.
4. **F5**: in `validate_ha_url`, reject hosts containing `@`, `?`, `#` explicitly with a clear message.
5. **F6 / F29**: lowercase scheme + host in `normalize_url`; preserve path case.
6. **F7**: add `SECONDS_PER_MINUTE`; add Hinnant attribution comment.
7. **F9**: add `compact: Option<bool>` to `get_states`; when true project to `{entity_id, state}`.
8. **F10**: change `domain_filter` to accept `String` or `Vec<String>` via untagged enum.
9. **F14**: validate `attributes.is_object()` in `set_state`.
10. **F15**: add `delete_state` action.
11. **F16**: add optional `variables` to `render_template`.
12. **F17**: raise `MAX_STATES` to 5000 (still token-bounded by `compact` mode) and surface `cap_kind: "user" | "hard"` in the truncated response.
13. **F28**: add cheap month/day range checks.

### `./tools-src/ha-tool/src/shell.rs`

14. **F18**: reject `\0` in `command`.
15. **F19**: tolerate unknown trailing content in `parse_exec_output` if at least one marker was seen.
16. **F20**: defer (see updated finding) — no implementation in this PR.
17. **F21**: include `_shell_fallback` metadata in the REST response when the shell path was tried and failed.

### `./tools-src/ha-tool/src/types.rs`

18. F2/F3/F4/F9/F10/F14/F15/F16: schema additions for new params.

### `./tools-src/ha-tool/src/lib.rs`

19. **F11**: memoize `schema()` and `description()` outputs in `OnceLock`.
20. **F12 / F30**: tighten `description()` to a single sentence + ssh hint.
21. **F15 / F16**: dispatch new variants.

### `./tools-src/ha-tool/Cargo.toml`

22. **F26**: add explanatory comment to `[workspace]`.

### `./skills/SKILL.md`

23. **F12**: keep verbose but remove the items duplicated by the description.
24. **F21**: document the silent-fallback behaviour explicitly.
25. **F22**: add `shell_status` to the on-session checklist.

### Tests

- Add `test_get_states_compact_projection` (round-trip: input attribute-rich → output flat).
- Add `test_render_template_truncation`.
- Add `test_validate_ha_url_rejects_userinfo` and `…_rejects_query_in_host`.
- Add `test_normalize_url_lowercases_scheme_and_host`.
- Add `test_parse_exec_output_tolerates_trailing_lines`.
- Add `test_is_shell_available_cached_per_request` (mock host stub).
- Add `test_validate_iso_prefix_rejects_invalid_month_day`.
- Update `test_validate_ha_url` with `http://attacker@192.168.1.1` cases.

### Edge cases & side effects

- **Schema cache (F11)**: WASM components can be invoked across multiple
  `execute` calls within the same instantiation — the host may pool
  instances. `OnceLock` per instance is safe; it only memoizes the
  serialized string. No cross-tenant data leak.
- **Compact mode (F9)**: agents that currently assume `attributes` is
  always present must opt in. This is backward-compatible because the
  default is `compact: false`.
- **Multi-domain filter (F10)**: `serde(untagged)` for
  `Option<StringOrVec>` adds schema complexity but keeps wire
  compatibility — single string still parses.
- **Lower-cased scheme (F29)**: passing `http://` (lowercase) to
  `host::http_request` is semantically identical; no behavior change for
  HA clients.
- **`delete_state` (F15)**: HA returns 200 + empty body on success;
  ensure the helper does not crash on empty response (it already
  tolerates this — `String::from_utf8` on empty bytes is `Ok("")`).
- **Cached probe (F20)**: each new `execute()` re-instantiates the
  module on most hosts; if the host pools, the cache is per-instance,
  not per-process — no staleness on long uptimes provided the gateway
  doesn't get uninstalled mid-execution (acceptable failure mode).

---

## Out of scope for this PR (separately tracked)

- Hand-written schema to shrink the ~6 KB schemars output (F27). High
  effort, marginal token win unless the host caches schema poorly.
- Configurable shell-command deny-list (F18). Risk of paternalism;
  defer until a real user requests it.
- Replacing `Cargo.toml`'s ad-hoc `[workspace]` with a proper
  `Cargo.lock`-tracked workspace at the repo root. Would need
  coordination with the IronClaw build pipeline (it currently calls
  `ironclaw tool install <path>` which expects a self-contained
  Cargo project).

---

## Implementation notes (applied in this PR)

### Code changes
- `./tools-src/ha-tool/src/api.rs`
  - F1: `StatesResponse` now exposes `matched` (post-filter) and `total` (unfiltered HA total) separately; `count` remains the actually-returned size.
  - F2: `get_history` accepts optional `end_time`, validated and appended as `&end_time=…`.
  - F3: `get_logbook` accepts optional `start_time` and `end_time`; falls back to `hours_back` only when both are absent.
  - F4 + F16: `render_template` accepts optional `variables` (must be JSON object) and `max_chars` (default 8 KiB, hard ceiling 16 KiB). Truncation is UTF-8-safe and appends a `…[truncated, N more bytes — pass max_chars to widen]` footer.
  - F5: `validate_ha_url` explicitly rejects `@`, `?`, `#` in the authority (defense-in-depth against userinfo/query/fragment confusion).
  - F6 + F29: `normalize_url` now lowercases scheme + authority while preserving path case.
  - F7: extracted `SECONDS_PER_MINUTE` constant; added Howard Hinnant attribution comment to `days_to_ymd` so future readers don't try to "fix" the magic constants.
  - F9: `get_states` accepts `compact: bool`; when true, projects each entity to `{entity_id, state, last_changed?}`, dropping the full attribute map for cheap discovery.
  - F10: `domain_filter` accepts a single string or an array of strings (`StringOrVec` with `serde(untagged)`); union-matches.
  - F14: `set_state` rejects `attributes` that aren't JSON objects with a clear error before round-tripping to HA.
  - F15: new `delete_state` action wired through to `DELETE /api/states/{entity_id}`; tolerates HA's empty 200 response.
  - F17: `MAX_STATES` raised to 5000; truncated responses now include `cap_kind: "user" | "hard"`.
  - F28: `validate_iso_prefix` adds cheap month (01-12) and day (01-31) range checks.
- `./tools-src/ha-tool/src/shell.rs`
  - F18: `shell_exec` rejects commands containing `\0`.
  - F19: `parse_exec_output` tolerates trailing lines after `(no output)` (forward-compatibility with future gateway footers).
  - F20: deferred (no caller batches multiple shell-aware actions into one `execute()`).
  - F21: alternative implemented — silent shell→REST fallback documented loudly in `./skills/SKILL.md` instead of changing the response wire format. `restart_ha` already uses the strict variant.
- `./tools-src/ha-tool/src/types.rs`
  - Added `StringOrVec` enum, `compact`, `end_time`, `variables`, `max_chars`, `DeleteState`, `start_time`/`end_time` for logbook; `StatesResponse` gained `matched` and `cap_kind`.
- `./tools-src/ha-tool/src/lib.rs`
  - F11: `schema()` and `description()` are memoized via `OnceLock` to avoid recomputation.
  - F12 + F30: `description()` tightened to one paragraph; mentions ssh hint and `compact` discovery shortcut.
  - F15 + F16: dispatch for new variants/parameters.
  - List shortcuts (`list_automations` / `list_scripts` / `list_scenes`) now use `compact: true` automatically (F13: keep + honor compact mode).
- `./tools-src/ha-tool/Cargo.toml` — F26 explanatory comment on the empty `[workspace]` block.
- `./skills/SKILL.md` — F12/F21/F22 documentation: `shell_status` workflow tip, silent-fallback heads-up, new `compact`/`max_chars`/`variables`/`end_time` parameters, `delete_state` action, and the new `matched`/`total`/`cap_kind` response fields.

### Tests
Added 5 regression tests, all green:
- `test_validate_iso_prefix_rejects_invalid_month_day`
- `test_validate_ha_url_rejects_userinfo_query_fragment`
- `test_normalize_url_lowercases_scheme_and_host`
- `test_parse_exec_output_tolerates_trailing_lines_after_no_output`
- `test_shell_exec_rejects_null_bytes`

### Test results
`cargo test --lib` — **33 passed, 0 failed** (28 pre-existing + 5 new).
`scripts/build.sh` — produces `dist/ha_tool.wasm` (557,307 bytes) cleanly.

### Out of scope (still tracked)
- F20: per-request shell-availability cache (deferred until a batch action lands).
- F21 wrapper variant: kept the documentation route to preserve the wire-compatible REST-pass-through shape. Revisit if users hit silent-fallback confusion in practice.
- F27: hand-written schema to shrink schemars output.
