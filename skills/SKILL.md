---
name: home-assistant
version: 0.4.1
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
  max_context_tokens: 3000
---

# Home Assistant via ha-tool

Every call requires `ha_url` — ask once, reuse. The sandbox enforces **HTTPS and public hostnames**: use `https://<id>.ui.nabu.casa` (Home Assistant Cloud) or a public DuckDNS/custom domain with TLS.

**IMPORTANT — Local HTTP instances** (`192.168.*`, `*.local`, `http://`): ha-tool **cannot reach these at all** — neither REST nor shell actions work because the sandbox blocks both the HTTP request and the WASM-to-WASM `tool_invoke` to remote-shell. Instead, use the native `shell` tool directly with `curl`:
```
shell: curl -s -H "Authorization: Bearer $HA_TOKEN" http://192.168.1.100:8123/api/states
```
See **Local HA via shell+curl** section below for the full pattern.

## Actions

**Discovery**: `get_status`, `get_config`, `get_services`, `get_states` (`domain_filter` string|array, `max_items`, `compact: true` for minimal output)

**Entity**: `get_state`, `set_state` (optional `attributes` object), `delete_state`, `call_service` (domain + service + optional data)

**Automations**: `list_automations`, `toggle_automation` (entity_id + enabled), `trigger_automation`

**Scripts/Scenes**: `list_scripts`, `run_script` (optional variables), `list_scenes`, `activate_scene`

**MQTT**: `mqtt_publish` (topic + payload, optional qos/retain)

**Modbus**: `modbus_write` (unit + address + value + write_type coil|holding, optional hub). See **Modbus Workflows** below for register scanning, PDF import, and config management.

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

## Shell Access via ha-tool (public HTTPS only)

ha-tool's shell actions (`shell_exec`, `shell_read_file`, `shell_write_file`, `shell_tail_file`, `ha_cli`) use WASM-to-WASM `tool_invoke("remote-shell")` internally. **This only works when the remote-shell gateway is reachable from the sandbox** — which requires HTTPS + public hostname, same as REST actions. For local HA instances, these actions will fail; use `shell` + `curl` instead (see below).

Pass `ssh` object (host, port, username, password or private_key_pem; optional session_id/gateway_port/insecure_ignore_host_key).

**REST+Shell** (SSH optional, auto-fallback to REST): `check_config`, `get_error_log`, `restart_ha`

**Shell-only** (SSH required): `shell_status` (probe once/session), `shell_exec` (needs user confirmation), `shell_read_file`, `shell_write_file` (32 KiB cap), `shell_tail_file`, `ha_cli`

**YAML workflow**: `shell_read_file` → modify → `shell_write_file` → `check_config` → `reload_automations`

## Local HA via shell+curl (when ha-tool cannot reach HA)

When HA is on a local/private network (`http://`, `192.168.*`, `*.local`), **do not use ha-tool** for REST or shell actions. Instead, call the HA REST API directly via the native `shell` tool:

```
shell: curl -s -H "Authorization: Bearer <TOKEN>" http://<HA_IP>:8123/api/<endpoint>
```

**Common patterns:**
- **Get states**: `curl -s -H "Authorization: Bearer $T" http://HA:8123/api/states`
- **Get single entity**: `curl -s -H "Authorization: Bearer $T" http://HA:8123/api/states/sensor.temperature`
- **Call service**: `curl -s -X POST -H "Authorization: Bearer $T" -H "Content-Type: application/json" -d '{"entity_id":"light.living_room"}' http://HA:8123/api/services/light/turn_on`
- **Check config**: `curl -s -X POST -H "Authorization: Bearer $T" -H "Content-Type: application/json" -d '{}' http://HA:8123/api/config/core/check_config`
- **Error log**: `curl -s -H "Authorization: Bearer $T" http://HA:8123/api/error_log`
- **Restart**: `curl -s -X POST -H "Authorization: Bearer $T" -H "Content-Type: application/json" -d '{}' http://HA:8123/api/services/homeassistant/restart`
- **Config entries**: `curl -s -H "Authorization: Bearer $T" http://HA:8123/api/config/config_entries/entry`
- **MQTT publish**: `curl -s -X POST -H "Authorization: Bearer $T" -H "Content-Type: application/json" -d '{"topic":"home/test","payload":"1"}' http://HA:8123/api/services/mqtt/publish`
- **Read file via SSH**: `shell: ssh user@HA_IP cat /config/configuration.yaml`
- **Write file via SSH**: pipe content through `ssh user@HA_IP tee /config/file.yaml`

Ask the user for `HA_IP`, `PORT`, and `TOKEN` once, then reuse across all calls.

## Modbus Workflows (requires SSH)

### 1. Scan a Modbus device for registers

Use `shell_exec` to probe registers. Prefer `modpoll` (install: `pip install modpoll`). Fall back to Python `pymodbus` one-liners.

**Holding registers** (function code 3 — the most common):
```
modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 4 <host>:<port>
```

**Input registers** (function code 4):
```
modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 3 <host>:<port>
```

**Coils** (function code 1, returns 0/1):
```
modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 0 <host>:<port>
```

**Discrete inputs** (function code 2, read-only 0/1):
```
modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 1 <host>:<port>
```

Pymodbus fallback (no install needed if HA uses pymodbus):
```
python3 -c "from pymodbus.client import ModbusTcpClient; c=ModbusTcpClient('<host>',<port>); c.connect(); r=c.read_holding_registers(<start>,<count>,slave=<unit_id>); print(r.registers if not r.isError() else r); c.close()"
```

Scan strategy: start with holding registers 0–99 in chunks of 50, then expand ranges based on responses. Registers that return errors are unimplemented — skip them.

### 2. Import registers from a PDF / datasheet

When the user provides a device PDF or register table:

1. Parse the register table — extract: register address, name, data type (int16/uint16/int32/float32), read/write, scale factor, unit
2. Map to HA Modbus YAML entries:
   - `holding`/`input` register → `sensors:` entry (read) or `switches:`/`climate:` (write)
   - `coil` → `switches:` or `binary_sensors:` entry
   - `data_type`: int16, uint16, int32, uint32, float32, float64, string
   - `scale`/`offset` for unit conversion (e.g. raw value × 0.1 = °C)
   - `scan_interval` for poll frequency
3. Generate the YAML stanzas
4. Read existing config: `shell_read_file path=/config/configuration.yaml` (or the modbus include file)
5. Merge new entries under the correct hub — never duplicate addresses
6. Write back: `shell_write_file` → `check_config` → `reload_core_config`

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

### 3. Diagnose & fix Modbus errors

1. `get_error_log tail_lines=100` — look for `Modbus` / `pymodbus` errors
2. `get_config_entries domain=modbus` — get the hub's `entry_id`
3. Common fixes:
   - **Timeout / connection refused**: check host:port reachability via `shell_exec` (`nc -z <host> <port>`)
   - **Illegal data address**: register doesn't exist on device — remove from config or fix address
   - **Slave failure**: device overloaded — increase `scan_interval` or reduce register count
   - **CRC error** (RTU): wiring/baud rate issue — check serial config
4. After config edits: `check_config` → `reload_config_entry entry_id=<id>`

## HA MCP Server

ha-tool covers the full REST surface (maintenance, reloads, MQTT, Modbus, raw state writes, error logs, restart). If the HA MCP Server integration is enabled, use both — MCP for conversational Assist entities, ha-tool for everything else.
