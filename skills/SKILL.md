---
name: home-assistant
version: 0.5.2
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
  max_context_tokens: 2000
---

# Home Assistant via ha-tool

Every call requires `ha_url` — ask once, reuse. `ha_url` must be HTTPS with a public hostname: `https://<id>.ui.nabu.casa` (Home Assistant Cloud) or a public DuckDNS/custom domain with TLS.

## Actions

**Discovery**: `get_status`, `get_config`, `get_services`, `get_states` (`domain_filter` string|array, `max_items`, `compact: true` for minimal output)

**Entity**: `get_state`, `set_state` (optional `attributes` object), `delete_state`, `call_service` (domain + service + optional data)

**Automations**: `list_automations`, `toggle_automation` (entity_id + enabled), `trigger_automation`

**Scripts/Scenes**: `list_scripts`, `run_script` (optional variables), `list_scenes`, `activate_scene`

**MQTT**: `mqtt_publish` (topic + payload, optional qos/retain)

**Modbus**: `modbus_write` (unit + address + value + write_type coil|holding, optional hub). See **Modbus Workflows** below for PDF import and error diagnosis.

**Templates**: `render_template` (Jinja2 template, optional variables/max_chars; default 8 KiB, max 16 KiB)

**History**: `get_history` (entity_id, optional start_time/end_time ISO 8601 or hours_back default 24), `get_logbook` (optional entity_id, same time params), `get_calendar_events` (entity_id + start + end ISO 8601)

**Events**: `fire_event` (event_type + optional event_data)

**Notifications**: `get_notifications`, `dismiss_notification`. Send: `call_service` domain=`notify` service=`mobile_app_<name>`

**System**: `check_config`, `get_error_log` (optional `tail_lines`), `restart_ha` (caution!), `reload_core_config`, `reload_automations`, `reload_scripts`, `reload_scenes`, `reload_themes`, `reload_config_entry` (requires entry_id), `get_config_entries` (optional `domain` filter — use to discover `entry_id` for `reload_config_entry`)

## Workflow

1. **Discover**: `get_states` with `domain_filter` + `compact: true` to find entity IDs. Fall back to `get_state` for full attributes.
2. **Control**: `call_service` for any HA service — lights, climate, media, covers, locks, notifications.
3. **History**: always pass `start_time`/`end_time` or `hours_back` to avoid pulling full history.
4. **Templates**: use `render_template` for complex server-side conditions.

## Modbus Workflows

> **Note**: ha-tool provides `modbus_write` only. Register scanning requires direct TCP access to the Modbus device — use the local extension (install from the `local/` directory in this repo) or SSH into the HA host manually. The workflows below use only ha-tool actions.

### 1. Import registers from a PDF / datasheet

When the user provides a device PDF or register table:

1. Parse the register table — extract: register address, name, data type (int16/uint16/int32/float32), read/write, scale factor, unit
2. Map to HA Modbus YAML entries:
   - `holding`/`input` register → `sensors:` entry (read) or `switches:`/`climate:` (write)
   - `coil` → `switches:` or `binary_sensors:` entry
   - `data_type`: int16, uint16, int32, uint32, float32, float64, string
   - `scale`/`offset` for unit conversion (e.g. raw value × 0.1 = °C)
   - `scan_interval` for poll frequency
3. Generate the YAML stanzas and show them to the user for review
4. User applies them to `/config/configuration.yaml` manually or via HA file editor
5. `check_config` → `reload_core_config`

Example generated entry:
```yaml
modbus:
  - name: hub1
    type: tcp
    host: 192.168.1.50
    port: 502
    sensors:
      - name: "Inverter Power"
        address: 100
        input_type: holding
        data_type: uint16
        scale: 0.1
        unit_of_measurement: "kW"
        device_class: power
        scan_interval: 30
```

### 2. Diagnose & fix Modbus errors

1. `get_error_log tail_lines=100` — look for `Modbus` / `pymodbus` errors
2. `get_config_entries domain=modbus` — get the hub's `entry_id`
3. Common fixes:
   - **Timeout / connection refused**: verify the device host:port is reachable from the HA host (user must check manually or use local extension)
   - **Illegal data address**: register doesn't exist on device — remove from config or fix address
   - **Slave failure**: device overloaded — increase `scan_interval` or reduce register count
   - **CRC error** (RTU): wiring/baud rate issue — check serial config
4. After config edits: `check_config` → `reload_config_entry entry_id=<id>`

## HA MCP Server

ha-tool covers the full REST surface (maintenance, reloads, MQTT, Modbus, raw state writes, error logs, restart). If the HA MCP Server integration is enabled, use both — MCP for conversational Assist entities, ha-tool for everything else.
