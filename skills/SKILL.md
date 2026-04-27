---
name: home-assistant
version: 0.3.1
description: Control Home Assistant — lights, climate, switches, automations, scripts, scenes, MQTT, Modbus, and system management via ha-tool
activation:
  keywords:
    - home assistant
    - homeassistant
    - light
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
    - lock
    - fan
    - alarm
    - notification
    - entity
  exclude_keywords:
    - memory
    - ironclaw routine
    - cron schedule
    - cron
    - commit
    - git
    - code review
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
  max_context_tokens: 2500
---

# Home Assistant Control via ha-tool

Every ha-tool call requires `ha_url` — the user's HA base URL (e.g. `http://homeassistant.local:8123`). Ask the user once, then include it on every call. Must be a private/local address: localhost, 192.168.*, 10.*, 172.16-31.*, *.local, *.lan, *.home, *.duckdns.org, *.nabu.casa.

## Actions Reference

**Discovery**: `get_status`, `get_config`, `get_services`, `get_states` (params: `domain_filter` string|array, `max_items`, `compact: true` for minimal output)

**Entity control**: `get_state`, `set_state` (with optional `attributes` object), `delete_state`, `call_service` (domain + service + optional data — most flexible action, works for any HA service)

**Automations**: `list_automations`, `toggle_automation` (entity_id + enabled), `trigger_automation`

**Scripts & Scenes**: `list_scripts`, `run_script` (with optional variables), `list_scenes`, `activate_scene`

**MQTT**: `mqtt_publish` (topic + payload, optional qos/retain)

**Modbus**: `modbus_write` (unit + address + value + write_type coil|holding, optional hub)

**Templates**: `render_template` (Jinja2 template, optional variables/max_chars; output capped at 8 KiB default, 16 KiB max)

**History**: `get_history` (entity_id, optional start_time/end_time ISO 8601 or hours_back default 24), `get_logbook` (optional entity_id, same time params), `get_calendar_events` (entity_id + start + end ISO 8601)

**Events**: `fire_event` (event_type + optional event_data)

**Notifications**: `get_notifications`, `dismiss_notification`. To send: `call_service` domain=`notify` service=`mobile_app_<name>`

**System**: `check_config`, `get_error_log` (optional `tail_lines`), `restart_ha` (caution!), `reload_core_config`, `reload_automations`, `reload_scripts`, `reload_scenes`, `reload_themes`, `reload_config_entry` (requires entry_id)

## Workflow

1. **Discover first**: `get_states` with `domain_filter` + `compact: true` to find entity IDs cheaply. Fall back to `get_state` for full attributes.
2. **Use call_service for anything**: lights, climate, media, covers, locks, notifications — any HA service works.
3. **Bound history queries**: always pass `start_time`/`end_time` or `hours_back` to avoid pulling full history.
4. **Templates**: use `render_template` for complex server-side conditions.

## Shell Access (optional, via remote-shell extension)

If the `ironclaw-remote-shell-extension` is installed, pass an `ssh` object (fields: host, port, username, password or private_key_pem, optional session_id/gateway_port/insecure_ignore_host_key) to enable shell-backed operations.

**Shell-aware REST actions** (SSH optional, auto-fallback to REST): `check_config`, `get_error_log`, `restart_ha` (strict — shell errors surfaced, no silent fallback)

**Shell-only actions** (SSH required): `shell_status` (probe availability — call once per session), `shell_exec` (arbitrary command, requires user confirmation), `shell_read_file`, `shell_write_file` (32 KiB cap), `shell_tail_file`, `ha_cli` (run `ha <args>`)

**YAML edit workflow**: `shell_read_file` → modify → `shell_write_file` → `check_config` → `reload_automations`

## Complementary: HA MCP Server

ha-tool covers the full REST surface (maintenance, reloads, MQTT, Modbus, raw state writes, error logs, restart). If the HA MCP Server integration is enabled, use both together — MCP handles conversational Assist entities, ha-tool handles everything else.
