---
name: home-assistant-local
version: 0.6.0
description: Control local Home Assistant via built-in http tool or shell+curl - lights, climate, switches, automations, scripts, scenes, MQTT, Modbus, and system management
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

# Home Assistant — Local Extension

## Tool Selection (IMPORTANT)

This extension supports two modes. At session start, determine which to use:

**Mode A — `http` tool (preferred)**: IronClaw's built-in `http` tool is always available in every context (CLI, server, routines, jobs). Requires `HTTP_ALLOW_LOCALHOST=true` in the IronClaw environment to allow HTTP and private-IP requests. If the `http` tool returns "only https URLs are allowed" or "private or local IPs are not allowed", tell the user to set `HTTP_ALLOW_LOCALHOST=true` and restart IronClaw.

**Mode B — `shell` tool (fallback)**: Uses `shell` with `curl`. Only available when `allow_local_tools = true` is set at IronClaw startup. If you get "Tool shell not found", switch to Mode A.

**Decision logic**: Try `http` first. If it fails due to SSRF/HTTPS restrictions, try `shell`. If neither works, tell the user which env vars to set.

## Setup

At session start, obtain the HA URL and token:

**If `shell` is available:**
```
shell: cat ~/.ironclaw/.ha_token
shell: cat ~/.ironclaw/.ha_url
```

**If only `http` is available:** Ask the user for their HA URL and long-lived access token. Reuse both for all subsequent calls.

## API Call Patterns

### Mode A — http tool

GET request:
```json
{"method": "GET", "url": "<HA_URL>/api/<endpoint>", "headers": [{"name": "Authorization", "value": "Bearer <TOKEN>"}]}
```

POST request:
```json
{"method": "POST", "url": "<HA_URL>/api/<endpoint>", "headers": [{"name": "Authorization", "value": "Bearer <TOKEN>"}, {"name": "Content-Type", "value": "application/json"}], "body": <JSON_OBJECT>}
```

### Mode B — shell tool

```
shell: curl -s -H "Authorization: Bearer <TOKEN>" <HA_URL>/api/<endpoint>
shell: curl -s -X POST -H "Authorization: Bearer <TOKEN>" -H "Content-Type: application/json" -d '<JSON>' <HA_URL>/api/<endpoint>
```

## Common Endpoints

**Discovery**: `GET /api/` (status), `GET /api/config` (config), `GET /api/services` (services), `GET /api/states` (all states)

**Entity**: `GET /api/states/<entity_id>`, `POST /api/states/<entity_id>` (set state), `DELETE /api/states/<entity_id>`

**Services**: `POST /api/services/<domain>/<service>` with JSON body `{"entity_id": "..."}` plus optional service data

**Automations**: filter `GET /api/states` for `automation.*` entities. Toggle: `POST /api/services/automation/turn_on` or `turn_off`. Trigger: `POST /api/services/automation/trigger`

**Scripts/Scenes**: `POST /api/services/script/turn_on` with `{"entity_id": "script.<name>"}`. Scenes: `POST /api/services/scene/turn_on`

**MQTT**: `POST /api/services/mqtt/publish` with `{"topic": "...", "payload": "..."}`

**Modbus**: `POST /api/services/modbus/write_register` or `write_coil` with `{"hub": "...", "unit": N, "address": N, "value": N}`

**Templates**: `POST /api/template` with `{"template": "{{ states('sensor.temp') }}"}`

**History**: `GET /api/history/period/<start_time>?filter_entity_id=<id>&end_time=<end>`. Logbook: `GET /api/logbook/<start_time>?entity=<id>`

**Calendar**: `GET /api/calendars/<entity_id>?start=<iso>&end=<iso>`

**Events**: `POST /api/events/<event_type>` with optional JSON body

**Notifications**: `GET /api/states` filtered for `persistent_notification.*`. Send: `POST /api/services/notify/mobile_app_<name>`. Dismiss: `POST /api/services/persistent_notification/dismiss` with `{"notification_id": "..."}`

**System**: `POST /api/config/core/check_config` with `{}`, `GET /api/error_log`, `POST /api/services/homeassistant/restart` with `{}`, `POST /api/services/homeassistant/reload_core_config` with `{}`

**Reload**: `POST /api/services/automation/reload`, `POST /api/services/script/reload`, `POST /api/services/scene/reload`, `POST /api/services/frontend/reload_themes`

**Config entries**: `GET /api/config/config_entries/entry` (all), `GET /api/config/config_entries/entry?domain=<domain>` (filtered). Use `entry_id` from response with `POST /api/config/config_entries/entry/<entry_id>/reload`

## Workflow

1. **Discover**: `GET /api/states` then filter response by domain prefix (e.g. `light.`, `sensor.`). With shell mode, use `jq` for filtering: `curl ... | jq '[.[] | select(.entity_id | startswith("light."))]'`. With http mode, parse the JSON response directly.
2. **Control**: `POST /api/services/<domain>/<service>` for any HA service
3. **History**: always pass time bounds to avoid pulling full history
4. **Templates**: use `POST /api/template` for complex server-side conditions

## File Access via SSH (requires shell tool)

- **Read config**: `shell: ssh user@HA_IP cat /config/configuration.yaml`
- **Write config**: pipe content through `shell: ssh user@HA_IP tee /config/file.yaml`
- **After config edits**: check config, then reload

## Modbus Workflows

### 1. Scan a Modbus device (requires shell tool)

Use SSH to probe registers. Prefer `modpoll` (install: `pip install modpoll`). Fall back to Python `pymodbus` one-liners.

**Holding registers** (function code 3):
```
ssh user@HA modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 4 <host>:<port>
```

**Input registers** (function code 4):
```
ssh user@HA modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 3 <host>:<port>
```

**Coils** (function code 1, returns 0/1):
```
ssh user@HA modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 0 <host>:<port>
```

**Discrete inputs** (function code 2, read-only 0/1):
```
ssh user@HA modpoll -m tcp -a <unit_id> -r <start> -c <count> -t 1 <host>:<port>
```

Pymodbus fallback (no install needed if HA uses pymodbus):
```
python3 -c "from pymodbus.client import ModbusTcpClient; c=ModbusTcpClient('<host>',<port>); c.connect(); r=c.read_holding_registers(<start>,<count>,slave=<unit_id>); print(r.registers if not r.isError() else r); c.close()"
```

Scan strategy: start with holding registers 0-99 in chunks of 50, then expand ranges based on responses. Registers that return errors are unimplemented.

### 2. Import registers from a PDF / datasheet

When the user provides a device PDF or register table:

1. Parse the register table - extract: register address, name, data type (int16/uint16/int32/float32), read/write, scale factor, unit
2. Map to HA Modbus YAML entries:
   - `holding`/`input` register -> `sensors:` entry (read) or `switches:`/`climate:` (write)
   - `coil` -> `switches:` or `binary_sensors:` entry
   - `data_type`: int16, uint16, int32, uint32, float32, float64, string
   - `scale`/`offset` for unit conversion (e.g. raw value x 0.1 = C)
   - `scan_interval` for poll frequency
3. Generate the YAML stanzas
4. If shell is available: read existing config via SSH, merge, write back, check config, reload
5. If shell is unavailable: show the YAML to the user for manual integration

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

1. `GET /api/error_log` - look for `Modbus` / `pymodbus` errors
2. `GET /api/config/config_entries/entry?domain=modbus` - get the hub's `entry_id`
3. Common fixes:
   - **Timeout / connection refused**: if shell available, check via `ssh user@HA nc -z <host> <port>`. Otherwise, ask user to verify connectivity manually.
   - **Illegal data address**: register doesn't exist on device - remove from config or fix address
   - **Slave failure**: device overloaded - increase `scan_interval` or reduce register count
   - **CRC error** (RTU): wiring/baud rate issue - check serial config
4. After config edits: check config -> reload config entry

## HA MCP Server

If the HA MCP Server integration is enabled, use both: MCP for conversational Assist entities, this extension for everything else (maintenance, reloads, MQTT, Modbus, error logs, restart).
