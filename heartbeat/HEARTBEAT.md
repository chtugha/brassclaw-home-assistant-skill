# Home Assistant Heartbeat Checklist

<!-- INSTALL_PREAMBLE: This file is a template. {{HA_URL}} placeholders are
     replaced automatically by scripts/install.sh during installation.
     If you are editing this file manually, replace every {{HA_URL}} with
     your Home Assistant base URL (e.g. http://192.168.1.100:8123). -->

IronClaw reads this file on every heartbeat tick (default: every 30 minutes).
It runs read-only health checks, detects problems, and proposes fixes.
No changes are made without your explicit confirmation.

## Connection Method (detect once per session)

`{{HA_URL}}` determines how to call the HA API:

- **Public HTTPS** (`https://*.nabu.casa`, `https://*.duckdns.org`): use `ha-tool` actions directly — the sandbox allows these.
- **Local/HTTP** (`http://`, `192.168.*`, `*.local`, `*.lan`): **do not use ha-tool** — the sandbox blocks it. Use the native `shell` tool with `curl`:
  ```
  curl -s -H "Authorization: Bearer <TOKEN>" {{HA_URL}}/api/<endpoint>
  ```
  Ask the user for their HA long-lived access token once, then reuse it for all calls in the session.

Detect the URL scheme on the first tick, then use the same method for all checks. The check names and logic below are identical — only the call mechanism differs. For local HA, map each `ha-tool` action to its equivalent `curl` call (see SKILL.md "Local HA via shell+curl").

## Confirmation Rules (MANDATORY)

- **NEVER** call `restart_ha`, `call_service`, `set_state`, `fire_event`,
  `toggle_automation`, `run_script`, `activate_scene`, `mqtt_publish`,
  `modbus_write`, `reload_config_entry`, `reload_core_config`, `reload_automations`,
  `reload_scripts`, `reload_scenes`, `reload_themes`, or `dismiss_notification`
  during a heartbeat tick without explicit user confirmation in the notification.
- Heartbeat ticks are read-only by default: they **detect** problems and
  **propose** remediations; the user confirms before anything is executed.
- If a proposed remediation is confirmed by the user, execute it in the next
  regular chat turn — not inside the heartbeat job.

## Read-only Checks (safe every tick)

- [ ] `ha-tool get_status ha_url={{HA_URL}}` — confirm HA is reachable. If the call
      fails or returns non-200, notify the user immediately with the error.
- [ ] `ha-tool check_config ha_url={{HA_URL}}` — validate HA configuration. If
      `result` is not `"valid"`, notify the user with the `errors` field.
- [ ] `ha-tool get_notifications ha_url={{HA_URL}}` — list persistent notifications.
      If any are present, summarize `title` + `message` + `notification_id`.
- [ ] `ha-tool get_error_log ha_url={{HA_URL}}` — fetch the error log.
      Report only NEW error/warning lines since the last tick
      (compare against `heartbeat/ha-last-log.md` in memory; `heartbeat/`
      is a workspace-relative directory — create it on the first tick if
      it does not yet exist).
- [ ] `ha-tool get_states ha_url={{HA_URL}} domain_filter=automation` —
      flag any automation whose `state` is `"unavailable"` or whose
      `attributes.last_triggered` is older than 30 days (possibly stuck).
- [ ] `ha-tool get_states ha_url={{HA_URL}} domain_filter=binary_sensor` —
      flag any `problem` or `battery_low` sensor that is `on`, and any
      `connectivity` sensor that is `off` (HA device class semantics:
      connectivity sensors are `on` when connected, `off` when disconnected).
- [ ] `ha-tool get_states ha_url={{HA_URL}} domain_filter=sensor` —
      flag any sensor in state `"unavailable"` or `"unknown"`.
<!-- update domain scan removed from 30-min heartbeat — covered by the
     weekly "ha-weekly-updates" routine in routines.md (too noisy here). -->

## Analysis & Proposal

- [ ] If any read-only check surfaced issues, write a concise summary to
      memory at `heartbeat/ha-latest.md` with:
      - `time`, `status` (ok|warn|error)
      - `findings` — list of `{entity_id, issue, severity}`
      - `proposed_remediations` — list of `{action, params, rationale}` drawn
        from the extension actions (e.g. `reload_config_entry`, `toggle_automation`,
        `call_service homeassistant reload_config_entry`).
- [ ] Save the raw error-log snapshot to `heartbeat/ha-last-log.md` so the
      next tick can diff against it.

## Notification

- [ ] Send a notification **only if** findings exist. Format:
      `HA heartbeat: N findings — [brief summary]. Propose: [list actions].
       Reply "apply <n>" to execute action n, or "ignore" to dismiss.`
- [ ] Do **not** send a notification if all checks pass — heartbeat is silent
      on healthy systems.

## Remediation Dispatch (executed only after user confirms in chat)

When the user replies with "apply N" or an equivalent confirmation, look up
the N-th proposed remediation from `heartbeat/ha-latest.md` and call the
corresponding `ha-tool` action with the stored params. Common remediations:

- Config edits were made externally → `reload_core_config` / `reload_automations`
  / `reload_scripts` / `reload_scenes` / `reload_themes`.
- Single integration is broken → `reload_config_entry entry_id=<id>`.
  Use `get_config_entries domain=<integration>` to discover the `entry_id`.
- Automation is stuck disabled → `toggle_automation entity_id=<id> enabled=true`.
- Stale sensor from integration restart → `reload_config_entry` (preferred)
  or `restart_ha` (last resort, always ask twice).

### Error-pattern → Action table

| Log pattern / symptom | Likely cause | Remediation action |
|---|---|---|
| `Modbus.*timeout` / `Modbus.*connection` | Modbus hub lost contact | `get_config_entries domain=modbus` → `reload_config_entry entry_id=<id>` |
| `MQTT.*disconnected` / `MQTT.*connection lost` | MQTT broker unreachable | `get_config_entries domain=mqtt` → `reload_config_entry entry_id=<id>` |
| `Zigbee.*failed` / `ZHA.*error` | Zigbee coordinator issue | `get_config_entries domain=zha` → `reload_config_entry entry_id=<id>` |
| `Z-Wave.*dead` / `Z-Wave.*timeout` | Z-Wave node or controller | `get_config_entries domain=zwave_js` → `reload_config_entry entry_id=<id>` |
| `Setup.*failed.*<integration>` | Integration setup failure | `get_config_entries domain=<integration>` → `reload_config_entry` |
| `Entity.*unavailable` (many at once) | Full integration outage | `get_config_entries` → identify domain → `reload_config_entry` |
| `check_config` returns errors | YAML syntax / schema error | Show errors to user; suggest SSH to inspect config YAML |
| Persistent notification present | HA system alert | Show to user; `dismiss_notification` after acknowledgement |

## Rate Limits

- Use at most 8 tool calls per heartbeat tick to stay within typical LLM
  budgets. Batch via `get_states` with `domain_filter` rather than looping
  individual `get_state` calls.

## Token Budget (target — 1024 tokens per tick)

This is an LLM-side guideline, not a runtime-enforced limit. Every
heartbeat tick should fit tool outputs + analysis + notification into
**~1024 tokens total**. Exceeding this budget degrades the next tick's
diff quality and may be truncated by the model.

Note: `tail_lines` and `max_items` reduce **LLM context usage**, not
network traffic — HA has no server-side tail parameter, so the full
payload is still fetched over HTTP before local trimming.

Enforce by:

- Cap `get_error_log` with `tail_lines` (never fetch the full log in a tick).
- Cap `get_states` with `max_items` when a domain is crowded.
- Summarize each check into ≤ 120 tokens before writing to memory.
- Notifications must be ≤ 400 characters; put details in `heartbeat/ha-latest.md`.
- Never include raw JSON bodies in memory writes — store flat key/value lines.

## Dynamic Profile Selection

Pick ONE profile at the start of each tick based on the agent's available
context budget, then apply the matching caps. Small LLMs stay lean; large
LLMs scan deeper. If unsure, use `standard`.

### minimal (≤ 2k-token context models)
- Run only: `get_status`, `check_config`, `get_notifications`.
- Skip state scans and error log entirely.
- Notification only on failure; budget ≤ 300 tokens.

### standard (4k–16k context, default)
- Run: `get_status`, `check_config`, `get_notifications`,
  `get_error_log tail_lines=40`.
- Run state scans with `max_items=30` per domain (automation, sensor, binary_sensor).
- Summaries ≤ 120 tokens each; total budget ≤ 1024 tokens.

### full (≥ 32k context)
- Run: all checks listed above + `get_error_log tail_lines=200`.
- `max_items=200` per state domain scan.
- May include short excerpts of error lines in the notification.
- Total budget may extend to 3072 tokens.
