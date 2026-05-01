# Investigation — ha-tool ↔ remote-shell integration audit

## Bug summary (updated — round 3)

The v0.5.0 refactor (commit `78b13fd`) removed `shell.rs` and all SSH
subsystem code from ha-tool, replacing it with a "local extension" that
tells the agent to use IronClaw's **built-in `shell` tool** with `curl`
for local HA instances.

Runtime error:
```
Tool 'shell' failed: Tool error: Tool shell not found.
```

## Root cause analysis (updated)

The IronClaw `shell` tool (`src/tools/builtin/shell.rs`, name `"shell"`)
is a **dev-domain tool** — it is only registered when
`allow_local_tools = true`:

| IronClaw mode | `allow_local_tools` default | `shell` available? |
|---|---|---|
| CLI (`ironclaw chat`) | `true` (Default impl) | Yes |
| Server / relay / env | `false` (`parse_bool_env`, default false) | **No** |

The local extension (`local/skills/SKILL.md`, `local/heartbeat/HEARTBEAT.md`,
`local/heartbeat/routines.md`) instructs the agent to call the `shell` tool
for every HA API interaction. When `shell` isn't registered, every call
produces the `ToolError::NotFound { name: "shell" }` error.

### Why other tool paths don't work for local HA

| Tool | Reaches local HA? | Reason |
|---|---|---|
| `ha-tool` WASM | No | WASM sandbox blocks HTTP to private IPs / localhost |
| `remote-shell` WASM | No | WASM sandbox blocks HTTP to 127.0.0.1 (gateway) |
| Built-in `http` | No | SSRF protection blocks private IPs (unless `HTTP_ALLOW_LOCALHOST=true`) |
| Built-in `shell` | **Yes** (CLI only) | Runs on host, can `curl` any address |

The only viable path for local HA is the `shell` tool, which requires
`allow_local_tools = true`. This is the default for CLI mode, but NOT
for server/headless deployments — exactly the mode most HA monitoring
users need.

## Affected components

- `./local/scripts/install.sh` — does not verify `shell` tool availability
- `./local/skills/SKILL.md` — assumes `shell` is always available, no
  fallback or prerequisite guidance
- `./local/heartbeat/HEARTBEAT.md` — same assumption
- `./local/heartbeat/routines.md` — same assumption
- `./scripts/install.sh` — post-install warning for local HA suggests
  `./local/scripts/install.sh` but doesn't mention the `allow_local_tools`
  requirement

## Proposed solution

### A. Install script: verify shell tool availability

Add a check to `local/scripts/install.sh` that runs
`ironclaw tool list 2>/dev/null | grep -q shell` and warns if the shell
tool isn't available, with instructions to either:
1. Set `ALLOW_LOCAL_TOOLS=true` in the IronClaw config
2. Or use the public HTTPS path with the main installer instead

### B. SKILL.md: add prerequisites section

Add a clear "Prerequisites" section to `local/skills/SKILL.md` that
explains the `shell` tool dependency and what to do when it's unavailable.
Include a graceful fallback: if `shell` isn't available, tell the user
the local extension can't function and suggest switching to the HTTPS
path.

### C. HEARTBEAT.md / routines.md: add prerequisites

Same prerequisite note in both heartbeat files so the agent doesn't
blindly attempt `shell` calls during heartbeat ticks.

### D. Main install script: improve local HA warning

The main `scripts/install.sh` warns local HA users to use the local
installer. Add a note that the local installer requires
`allow_local_tools = true`.

## Edge cases

- Users running `ironclaw chat` (CLI mode) should not be affected — the
  `shell` tool is available by default.
- The install script check is advisory — if the user configures
  `allow_local_tools` after install, the extension starts working without
  reinstalling.
- Token storage in `~/.ironclaw/.ha_token` is a plaintext file, not in
  IronClaw's encrypted secret store. This is by design for the local
  extension (no WASM tool = no `ironclaw tool auth`), but should be
  noted in the prerequisites.

## Implementation notes (round 3)

Applied fixes for the `Tool shell not found` issue:

- **`./local/scripts/install.sh`**: Added pre-flight check that runs
  `ironclaw tool list | grep shell` to detect whether the `shell` tool
  is registered. If not found, displays a clear warning explaining:
  - The local extension requires `allow_local_tools = true`
  - CLI mode has it by default, server mode needs `ALLOW_LOCAL_TOOLS=true`
  - Alternative: use HTTPS remote extension
  Prompts user to confirm before continuing. Post-install summary also
  shows a reminder if `shell` was not detected.

- **`./local/skills/SKILL.md`**: Added "Prerequisites" section at the top
  explaining the `shell` tool requirement, when it's available, and what
  the agent should do if it gets "Tool shell not found" (report to user
  with fix instructions, do not retry).

- **`./local/heartbeat/HEARTBEAT.md`**: Added "Prerequisites" section
  instructing the agent to skip all checks and notify the user if the
  `shell` tool is unavailable.

- **`./local/heartbeat/routines.md`**: Added prerequisite note explaining
  that all routines require `allow_local_tools = true` and will fail
  without it.

- **`./scripts/install.sh`**: Updated the local HA warning to mention
  that the local installer requires the `shell` tool and
  `ALLOW_LOCAL_TOOLS=true` for server mode.

## Test results (round 3)

- `cargo test --lib` — **23 passed, 0 failed** (ha-tool tests)
- `cargo build --target wasm32-wasip2 --release` — succeeds
- `bash -n local/scripts/install.sh` — syntax OK
- `bash -n scripts/install.sh` — syntax OK
