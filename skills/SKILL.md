---
name: home-assistant
version: 0.2.0
description: Control Home Assistant — lights, climate, switches, automations, scripts, scenes, MQTT, Modbus, and system management via ha-tool
activation:
  keywords:
    - home assistant
    - homeassistant
    - light
    - lights
    - switch
    - thermostat
    - climate
    - temperature
    - automation
    - scene
    - script
    - sensor
    - smart home
    - mqtt
    - modbus
    - cover
    - blind
    - lock
    - fan
    - alarm
    - media player
    - notify
    - notification
    - entity
  patterns:
    - "turn (on|off|toggle).*(light|switch|fan|plug|outlet)"
    - "(run|trigger|enable|disable).*automation"
    - "(publish|send).*mqtt"
    - "modbus.*(read|write)"
    - "(activate|set).*scene"
  tags:
    - home-automation
    - iot
    - smarthome
  max_context_tokens: 3000
---

# Home Assistant Control

You have access to `ha-tool` which controls the user's Home Assistant instance via its REST API.

## Important: ha_url Parameter

Every ha-tool call requires `ha_url` — the base URL of the user's HA instance (e.g., `http://homeassistant.local:8123`).

- Ask the user for their HA URL if you don't know it yet.
- Once known, include `ha_url` in every ha-tool call for the rest of the conversation.
- `ha_url` must point to a private/local address: `localhost`, `127.0.0.1`, `192.168.*`, `10.*`, `172.16-31.*`, `*.local`, `*.internal`, `*.lan`, `*.home`, `*.duckdns.org`, or `*.nabu.casa`.
- Common formats: `http://homeassistant.local:8123`, `http://192.168.x.x:8123`, `https://myha.duckdns.org`

## Available Actions

### Discovery
- `get_status` — Check if HA is reachable
- `get_config` — Get HA configuration (version, location, units)
- `get_states` — List all entities. Parameters:
  - `domain_filter` — single domain (`"light"`) or array (`["light","switch","sensor"]`) to union-match
  - `max_items` — caps the returned list for small context budgets
  - `compact: true` — projects each entity to `{entity_id, state, last_changed?}`, dropping the full attribute map (huge token savings during discovery; fall back to `get_state` when you need attributes)
  - Response: `{entities, count, matched, total, truncated?, cap_kind?}` where `matched` is the post-filter count, `total` is the unfiltered HA total, and `cap_kind` is `"user"` or `"hard"` when truncated
- `get_services` — List all available service domains and their services

### Entity Control
- `get_state` — Get current state of a specific entity
- `set_state` — Set entity state directly (`attributes` must be a JSON object if provided)
- `delete_state` — Remove a manually-created state (HA `DELETE /api/states/{entity_id}`)
- `call_service` — Call any HA service (most flexible action)

### Automations
- `list_automations` — List all automations
- `toggle_automation` — Enable/disable an automation
- `trigger_automation` — Trigger an automation manually

### Scripts & Scenes
- `list_scripts` / `run_script` — List and run scripts (with optional variables)
- `list_scenes` / `activate_scene` — List and activate scenes

### MQTT
- `mqtt_publish` — Publish a message to an MQTT topic (with optional qos, retain)

### Modbus
- `modbus_write` — Write to Modbus coils (boolean) or holding registers (number)

### Templates
- `render_template` — Render a Jinja2 template on the HA server. Optional `variables` (object) is forwarded to HA. Output is capped (default 8 KiB, hard ceiling 16 KiB); raise via `max_chars`. When truncated, the response ends with `…[truncated, N more bytes — pass `max_chars` to widen]`.

### History & Logs
- `get_history` — Entity state history. Bound the window with `start_time` and/or `end_time` (ISO 8601). When neither is provided, falls back to `hours_back` (default 24, max 8760).
- `get_logbook` — Event logbook. Same time-window options as `get_history`; optional `entity_id` filter.
- `get_calendar_events` — Calendar events (requires `start` and `end` in ISO 8601)

### Events
- `fire_event` — Fire a custom event on the HA event bus

### Notifications
- `get_notifications` — List persistent notifications
- `dismiss_notification` — Dismiss a notification by ID
- To **send** a notification, use `call_service` with domain `notify` and the target service (e.g., `mobile_app_my_phone`)

### System & Reloads
- `check_config` — Validate HA configuration
- `get_error_log` — View the HA error log (optional `tail_lines` returns only the last N lines — reduces LLM context usage, not network traffic, since HA has no server-side tail)
- `restart_ha` — Restart Home Assistant (use with caution!)
- `reload_core_config` — Reload core `configuration.yaml` without restart
- `reload_automations` — Reload automations after YAML edits
- `reload_scripts` — Reload scripts after YAML edits
- `reload_scenes` — Reload scenes after YAML edits
- `reload_themes` — Reload frontend themes
- `reload_config_entry` — Reload an integration config entry (requires `entry_id`)

## Complementary: Home Assistant MCP Server

If your HA instance has the [MCP Server integration](https://www.home-assistant.io/integrations/mcp_server/) enabled, IronClaw can connect to it directly as a native MCP client for Assist-exposed entities (conversational control). `ha-tool` covers the full REST surface (maintenance, reloads, automations, raw state writes, MQTT, Modbus, error logs, restart) which HA's MCP server does not expose. Use both together for maximum coverage.

## Shell-Backed Actions (optional, via `remote-shell` extension)

If the user has installed the `ironclaw-remote-shell-extension`, `ha-tool` can perform operations that the REST API cannot (YAML editing, real log tailing, `ha` supervisor CLI). Pass an `ssh` object to any shell-aware action. If the remote-shell extension is not installed or the shell call fails, `ha-tool` logs a warning and falls back to the REST API automatically.

> **Heads-up — silent shell→REST fallback.** When `check_config` or `get_error_log` is called with `ssh` and the shell path fails (bad credentials, gateway not running, transient network error), the tool currently logs a warning at level `Warn` and silently routes the call to the REST API. The user-visible response is the REST result with no fallback marker. If you suspect a fallback happened (the response shape doesn't match the shell-backed expectation), check the host log and call `shell_status` to verify gateway availability. `restart_ha` is the exception: it uses a *strict* shell path that surfaces shell errors instead of silently falling back to REST.

### SshConfig schema
```json
{
  "ssh": {
    "session_id": "optional — reuse an existing session",
    "host": "homeassistant.local",
    "port": 22,
    "username": "root",
    "password": "optional",
    "private_key_pem": "optional",
    "host_key_fingerprint": "optional",
    "insecure_ignore_host_key": false,
    "gateway_port": 0
  }
}
```

### Shell-aware REST actions (SSH optional)
- `check_config` — prefers `ha core check` over SSH when `ssh` is provided
- `get_error_log` — prefers `tail -n <tail_lines> <log_path>` over SSH (path defaults to `/config/home-assistant.log`)
- `restart_ha` — prefers `ha core restart` over SSH

### Shell-only actions (SSH required)
- `shell_status` — check whether the remote-shell extension is installed/reachable. Accepts an optional `gateway_port` (match it to the value used in your `ssh` config when non-default). Returns a JSON object including `remote_shell_available` (bool); inspect this on a fresh session before opting into shell-aware actions.
- `shell_exec` — run an arbitrary shell command (`command`, optional `timeout_secs`). **Intentionally unrestricted; runs with the privileges of the SSH user (typically `root` on Home Assistant OS / Supervised, but may be a regular user on a plain Linux install).** Only invoke with explicit user confirmation of the exact command for each call.
- `shell_read_file` — read a file via `cat` (`path`)
- `shell_write_file` — atomically write a file via `base64 -d` (`path`, `content`). Capped at 32 KiB per call so the base64-encoded command stays within the gateway's command-length budget; chunk larger writes manually.
- `shell_tail_file` — tail last N lines (`path`, `lines`)
- `ha_cli` — run `ha <args>` (e.g. `core check`, `core restart`, `core logs`, `addons list`)

### Typical YAML-edit workflow
1. `shell_read_file` to fetch `/config/automations.yaml`
2. Modify content locally/in agent memory
3. `shell_write_file` to persist the new content
4. `check_config` to validate
5. `reload_automations` (REST) to apply without restart

## Limitations

- Real-time WebSocket event subscription is not supported (WASM sandbox is request/response only). Use `get_history` / `get_logbook` polling for monitoring.
- Without the `remote-shell` extension, direct YAML file editing is out of scope. Use `reload_*` actions after the user edits files, or call the File Editor addon's own services via `call_service`.

## Workflow Tips

0. **On a fresh session that intends to use shell-aware actions**, call `shell_status` once and cache the result — it tells you whether `ssh`-backed paths will actually be taken (vs silently falling back to REST).
1. **Start with discovery**: Use `get_states` with domain_filter (single string or array) and `compact: true` to find entity IDs cheaply before operating on them.
2. **Use call_service for anything**: Any HA service can be called directly — lights, climate, media, covers, locks, etc.
3. **MQTT and Modbus**: These use HA's integration services, so HA must have the MQTT/Modbus integrations configured.
4. **Templates**: Use `render_template` to evaluate complex conditions or calculations on the HA server.
5. **Automations**: List them first, then enable/disable/trigger as needed. To edit automation YAML, you'll need file access to HA's config directory.

## Example Calls

```json
{"action": "get_states", "ha_url": "http://homeassistant.local:8123", "domain_filter": "light"}
```

```json
{"action": "call_service", "ha_url": "http://homeassistant.local:8123", "domain": "light", "service": "turn_on", "data": {"entity_id": "light.living_room", "brightness": 200}}
```

```json
{"action": "mqtt_publish", "ha_url": "http://homeassistant.local:8123", "topic": "home/command", "payload": "restart"}
```

```json
{"action": "call_service", "ha_url": "http://homeassistant.local:8123", "domain": "notify", "service": "mobile_app_my_phone", "data": {"message": "Hello from IronClaw"}}
```
