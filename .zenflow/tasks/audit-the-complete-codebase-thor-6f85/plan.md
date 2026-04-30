### [x] Step: Implement P1–P5 fixes from audit

- [x] P1: Added `get_config_entries` action (GET /api/config/config_entries/entry) with optional `domain` filter — types.rs, api.rs, lib.rs
- [x] P2: Fixed HEARTBEAT.md connectivity sensor logic inversion (was flagging `on`; now correctly flags `off` for connectivity, `on` for problem/battery_low)
- [x] P3: Removed `update` domain scan from 30-min heartbeat (already covered by weekly `ha-weekly-updates` routine)
- [x] P4: Added error-pattern-to-action remediation table in HEARTBEAT.md (Modbus, MQTT, Zigbee, Z-Wave, setup failures, unavailable entities, config errors, notifications)
- [x] P5: Synced ha-tool.capabilities.json (max_items 5000, compact, delete_state, template variables, end_time, get_config_entries)
- [x] Updated SKILL.md and TOOL_DESCRIPTION with get_config_entries
- [x] Added test_get_config_entries_domain_validation test (40 tests total, all pass)

### [x] Step: P6 analysis — Modbus undeclared registers

Analysis provided in chat. Added Modbus Workflows section to SKILL.md (register scanning, PDF import, error diagnosis).

### [x] Step: Deep code review — bugs, security, dead code

- [x] F1: Fixed modbus_write validation — accept arrays for multi-register/coil writes (HA supports value: int|list[int] and bool|list[bool])
- [x] F2: Fixed capabilities.json `always_required` — was incorrectly claiming ha_url is always required (false for 6 shell-only actions)
- [x] F3: Version bump — Cargo.toml + capabilities.json 0.2.0 → 0.3.0 (was stale across 3 audit rounds)
- [x] F4: Added warning log when `get_error_log` log_path is silently ignored in REST fallback
- [x] F5: Added test_modbus_write_value_validation test (41 tests total, all pass)
- Verified: all HTTP paths go through validate_ha_url, all API paths start with /api/ (sandbox allowlist compliant), no dead code, no stubs, all magic numbers are named constants

### [x] Step: Critical allowlist/credential fix — IronClaw upstream verification

Verified IronClaw upstream source (`staging` branch) for both host matching functions:
- `EndpointPattern::host_matches()` in `src/tools/wasm/capabilities.rs` — only supports exact match or `*.suffix` wildcard
- `host_matches_pattern()` in `src/secrets/types.rs` — same logic (exact match, port-stripping, `*.suffix`)
- **Neither function treats bare `"*"` as a catch-all** — it only matches the literal string `"*"` as a hostname

**Critical bugs found and fixed:**
- [x] `ha-tool.capabilities.json` allowlist `"host": "*"` matched no real hostname → every HTTP request would fail with `HostNotAllowed`
- [x] `ha-tool.capabilities.json` credentials `"host_patterns": ["*"]` matched no real hostname → bearer token was never injected
- [x] Replaced `"host": "*"` with `["*.nabu.casa", "*.duckdns.org"]` (the two public domain families IronClaw's `EndpointPattern::host_matches` supports)
- [x] Replaced `"host_patterns": ["*"]` with `["*.nabu.casa", "*.duckdns.org"]` (matching `host_matches_pattern` logic)
- [x] Verified consistency with `validate_ha_url` which accepts exactly those two public domain families
- [x] Version bump: 0.3.x → 0.4.0 across Cargo.toml, capabilities.json, SKILL.md
- [x] All 41 tests pass

### [x] Step: DuckDNS + Let's Encrypt automated setup

- [x] Created `scripts/setup-duckdns.sh` — standalone helper that installs/configures the HA DuckDNS add-on via Supervisor API with Let's Encrypt enabled
- [x] Integrated HTTPS setup recommendation into `install.sh` post-install output (shown when HA URL is local/HTTP)
- Script flow: prompts for HA URL + token + DuckDNS subdomain + token → checks Supervisor availability → installs add-on → configures domain + Let's Encrypt → starts add-on → saves new HTTPS URL
- Falls back to certbot instructions for non-HA-OS installs

### [x] Step: Necessity audit — remove dead shell code (v0.5.0)

Assessed every component for necessity given the sandbox architecture: WASM tools cannot reach local HA instances (sandbox blocks HTTP + private IPs), and the `tool_invoke("remote-shell")` path is equally blocked (WASM-to-WASM HTTP to localhost:9022). Only two paths work: (1) ha-tool REST for public HTTPS HA, (2) native `shell` tool + `curl` for local HA.

**Removed (833 lines of dead code):**
- [x] Deleted `shell.rs` entirely (721 lines) — SshConfig, try_shell, try_shell_strict, ensure_session, parse_connect_response, shell_exec, parse_exec_output, read_file, write_file, tail_file, ha_cli, shell_status, is_shell_available, b64_encode, validate_path, is_sandbox_error, all 19 shell tests
- [x] Removed 6 shell-only action variants from types.rs: ShellStatus, ShellExec, ShellReadFile, ShellWriteFile, ShellTailFile, HaCli
- [x] Removed SSH fallback from check_config, get_error_log, restart_ha in api.rs — now pure REST
- [x] Removed SshConfig fields from CheckConfig, GetErrorLog, RestartHa action variants
- [x] Removed `tool_invoke` section from capabilities.json (remote-shell alias no longer needed)
- [x] Removed "Shell Access via ha-tool" section from SKILL.md
- [x] Removed stale log_path warning and DEFAULT_HA_LOG_PATH/DEFAULT_SHELL_TAIL_LINES constants

**Updated:**
- [x] capabilities.json: simplified description, discovery_summary, and notes
- [x] SKILL.md: removed shell-via-ha-tool docs, updated Modbus workflows to reference native shell+SSH, reduced max_context_tokens 3000→2500
- [x] HEARTBEAT.md: replaced shell_read_file reference with SSH
- [x] lib.rs TOOL_DESCRIPTION: simplified, removed shell action references
- [x] install.sh: updated post-install message (native shell+curl, not SSH shell path)
- [x] README.md: replaced SSH Shell Access section with Local HA Instances section, updated project structure (removed shell.rs, added setup-duckdns.sh)
- [x] Version bump: 0.4.1 → 0.5.0 across Cargo.toml, capabilities.json, SKILL.md
- [x] All 23 tests pass, WASM build succeeds (471.8 KiB)
