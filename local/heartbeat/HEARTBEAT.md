<!-- VARIANT: local -->
<!-- INSTALL_PREAMBLE: This file is a template. {{HA_URL}} and {{HA_TOKEN}}
     placeholders are replaced automatically by local/scripts/install.sh
     during installation. If you are editing this file manually, replace
     every {{HA_URL}} with your HA base URL and {{HA_TOKEN}} with your
     long-lived access token. -->

IronClaw reads this file on every heartbeat tick (default: every 30 minutes).
It runs read-only health checks, detects problems, and proposes fixes.
No changes are made without your explicit confirmation.

## Tool Selection

This heartbeat supports two modes:

**Mode A — `http` tool (preferred)**: Always available. Requires
`HTTP_ALLOW_LOCALHOST=true` in the IronClaw environment.

**Mode B — `shell` tool (fallback)**: Requires `allow_local_tools = true`
at IronClaw startup.

At tick start, try the `http` tool first. If it returns an SSRF/HTTPS
restriction error, fall back to `shell`. If neither works, send a single
notification: "HA heartbeat: no usable tool. Set HTTP_ALLOW_LOCALHOST=true
and restart IronClaw." Then skip all checks.

## API Call Method

At tick start, obtain the token and URL:
- If `shell` is available: `shell: cat ~/.ironclaw/.ha_token` and
  `shell: cat ~/.ironclaw/.ha_url`
- If only `http` is available: the token and URL below were injected by
  the install script — use them directly.

**http tool** (Mode A):
```json
{"method": "GET", "url": "{{HA_URL}}/api/<endpoint>", "headers": [{"name": "Authorization", "value": "Bearer {{HA_TOKEN}}"}]}
```

**shell tool** (Mode B):
```
curl -s -H "Authorization: Bearer {{HA_TOKEN}}" {{HA_URL}}/api/<endpoint>
```

## Confirmation Rules (MANDATORY)

- **NEVER** call any write endpoint (`POST /api/services/*`, `DELETE`,
  state writes, reload, restart) during a heartbeat tick without explicit
  user confirmation in the notification.
- Heartbeat ticks are read-only by default: they **detect** problems and
  **propose** remediations; the user confirms before anything is executed.
- If a proposed remediation is confirmed by the user, execute it in the next
  regular chat turn — not inside the heartbeat job.

## Read-only Checks (safe every tick)

- [ ] `GET /api/` — confirm HA is reachable. If the call fails or returns
      non-200, notify the user immediately with the error.
- [ ] `POST /api/config/core/check_config` with `{}` — validate HA
      configuration. If `result` is not `"valid"`, notify the user with
      the `errors` field.
- [ ] `GET /api/states` filtered for `persistent_notification.*` — list
      persistent notifications. If any are present, summarize title +
      message + notification_id.
- [ ] `GET /api/error_log` — fetch the error log. Report only NEW
      error/warning lines since the last tick (compare against
      `heartbeat/ha-last-log.md` in memory; `heartbeat/` is a
      workspace-relative directory — create it on the first tick if
      it does not yet exist).
- [ ] `GET /api/states` filtered for `automation.*` — flag any automation
      whose `state` is `"unavailable"` or whose `last_triggered` attribute
      is older than 30 days (possibly stuck).
- [ ] `GET /api/states` filtered for `binary_sensor.*` — flag any `problem`
      or `battery_low` sensor that is `on`, and any `connectivity` sensor
      that is `off` (HA device class semantics: connectivity sensors are
      `on` when connected, `off` when disconnected).
- [ ] `GET /api/states` filtered for `sensor.*` — flag any sensor in state
      `"unavailable"` or `"unknown"`.

For domain filtering:
- **http mode**: `GET /api/states` returns JSON directly — filter the response array for matching `entity_id` prefixes.
- **shell mode**: pipe through `jq`: `curl -s -H "Authorization: Bearer {{HA_TOKEN}}" {{HA_URL}}/api/states | jq '[.[] | select(.entity_id | startswith("automation."))]'`

## Analysis & Proposal

- [ ] If any read-only check surfaced issues, write a concise summary to
      memory at `heartbeat/ha-latest.md` with:
      - `time`, `status` (ok|warn|error)
      - `findings` — list of `{entity_id, issue, severity}`
      - `proposed_remediations` — list of `{action, params, rationale}` drawn
        from HA service calls (e.g. reload config entry, toggle automation).
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
corresponding endpoint. Common remediations:

- Config edits were made externally: `POST /api/services/homeassistant/reload_core_config` /
  `POST /api/services/automation/reload` / `POST /api/services/script/reload` /
  `POST /api/services/scene/reload`
- Single integration is broken: `POST /api/config/config_entries/entry/<entry_id>/reload`.
  Use `GET /api/config/config_entries/entry?domain=<integration>` to discover the `entry_id`.
- Automation is stuck disabled: `POST /api/services/automation/turn_on` with `{"entity_id": "<id>"}`.
- Stale sensor from integration restart: reload config entry (preferred)
  or `POST /api/services/homeassistant/restart` with `{}` (last resort, always ask twice).

### Error-pattern -> Action table

| Log pattern / symptom | Likely cause | Remediation |
|---|---|---|
| `Modbus.*timeout` / `Modbus.*connection` | Modbus hub lost contact | `GET /api/config/config_entries/entry?domain=modbus` -> reload entry |
| `MQTT.*disconnected` / `MQTT.*connection lost` | MQTT broker unreachable | `GET /api/config/config_entries/entry?domain=mqtt` -> reload entry |
| `Zigbee.*failed` / `ZHA.*error` | Zigbee coordinator issue | `GET /api/config/config_entries/entry?domain=zha` -> reload entry |
| `Z-Wave.*dead` / `Z-Wave.*timeout` | Z-Wave node or controller | `GET /api/config/config_entries/entry?domain=zwave_js` -> reload entry |
| `Setup.*failed.*<integration>` | Integration setup failure | `GET /api/config/config_entries/entry?domain=<integration>` -> reload |
| `Entity.*unavailable` (many at once) | Full integration outage | identify domain -> reload config entry |
| `check_config` returns errors | YAML syntax / schema error | Show errors to user; suggest SSH to inspect config YAML |
| Persistent notification present | HA system alert | Show to user; dismiss after acknowledgement |

## Rate Limits

- Use at most 8 API calls per heartbeat tick to stay within typical LLM
  budgets. Combine state queries by filtering a single `GET /api/states`
  response rather than individual entity fetches.

## Token Budget (target — 1024 tokens per tick)

This is an LLM-side guideline, not a runtime-enforced limit. Every
heartbeat tick should fit tool outputs + analysis + notification into
**~1024 tokens total**.

Enforce by:

- Extract only relevant fields from state responses (use `jq` in shell mode, or parse JSON directly in http mode).
- Limit error log output to the last 40 lines.
- Summarize each check into <= 120 tokens before writing to memory.
- Notifications must be <= 400 characters; put details in `heartbeat/ha-latest.md`.
- Never include raw JSON bodies in memory writes — store flat key/value lines.

## Dynamic Profile Selection

Pick ONE profile at the start of each tick based on the agent's available
context budget, then apply the matching caps. If unsure, use `standard`.

### minimal (<= 2k-token context models)
- Run only: status check, check_config, notifications.
- Skip state scans and error log entirely.
- Notification only on failure; budget <= 300 tokens.

### standard (4k-16k context, default)
- Run: status, check_config, notifications, error log (last 40 lines).
- Run state scans for automation, sensor, binary_sensor domains.
- Summaries <= 120 tokens each; total budget <= 1024 tokens.

### full (>= 32k context)
- Run: all checks listed above + error log (last 200 lines).
- No item caps on state domain scans.
- May include short excerpts of error lines in the notification.
- Total budget may extend to 3072 tokens.
