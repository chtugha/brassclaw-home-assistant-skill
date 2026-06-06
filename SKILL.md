---
name: home-assistant
version: 0.6.0
description: Control Home Assistant via MCP (Monolithic version)
metadata:
  openclaw:
    envVars:
      - name: HA_URL
        required: true
        description: The Home Assistant URL (e.g. http://192.168.1.100:8123)
      - name: HA_TOKEN
        required: true
        description: Long-lived Access Token generated in Home Assistant Profile
credentials:
  - name: HA_URL
    provider: homeassistant
    location:
      type: header
      name: X-HA-URL
    hosts:
      - "*"
    setup_instructions: "Enter the Home Assistant URL (e.g., http://192.168.19.37:8123)"
  - name: HA_TOKEN
    provider: homeassistant
    location:
      type: bearer
    hosts:
      - "*"
    setup_instructions: "Enter your Long-lived Access Token generated in your Home Assistant profile"
activation:
  keywords:
    - home assistant
    - homeassistant
    - light
    - switch
    - thermostat
    - climate
    - temperature
  patterns:
    - "turn (on|off|toggle).*(light|switch|fan|plug|outlet)"
    - "set up (home assistant|homeassistant) mcp"
    - "install (home assistant|homeassistant) mcp"
  tags:
    - home-automation
  max_context_tokens: 3000
---

# Home Assistant MCP (Monolithic)

> **💡 Optimization Note**: To reduce token context size and improve instruction-following accuracy, you can install the individual split subskills from this repository instead:
> - **Control & Search**: `./skills/control-search/SKILL.md`
> - **Diagnostics**: `./skills/diagnostics/SKILL.md`
> - **Config & Modbus**: `./skills/config-modbus/SKILL.md`

Use these tools to manage Home Assistant:
1. **`homeassistant_ha_search_entities(query, domain=None)`**: Search for entities, status, or updates.
2. **`homeassistant_ha_control(entity_id, action, value=None)`**: Control devices (turn_on/off, toggle, set_value).
3. **`homeassistant_ha_get_diagnostics()`**: Check health, logs, or updates.
4. **`homeassistant_ha_edit_config(action, file=None, old_string=None, new_string=None, offset=None, limit=None)`**: Read or patch configuration files.
5. **`homeassistant_ha_probe_modbus(register_type, address, host=None, port=None, unit_id=None, count=None)`**: Directly probe Modbus TCP registers.

**Instructions for Common Requests**:
- **Updates / System Health**: Call `homeassistant_ha_get_diagnostics()`. Also search for `update` entities with `homeassistant_ha_search_entities(query="update")`.
- **Status of Devices**: Search for the device name/type first to inspect its current state.
- **Control Devices**: Search to find the exact `entity_id` first, then call control action with that ID.
- **Editing Configuration**: Read or patch configuration using `homeassistant_ha_edit_config`. Make sure to search the configuration layout first using read.
- **Modbus Regs**: Probe registers directly with `homeassistant_ha_probe_modbus`.

## Self-Installation and Registration (For the Agent)

If the user asks you to set up, install, or register the Home Assistant MCP server, follow these exact steps using your `shell` tool:
1. **Clone & Compile**:
   Clone the repository:
   ```bash
   git clone https://github.com/chtugha/brassclaw-home-assistant-skill /tmp/brassclaw-home-assistant-skill
   ```
   Compile the MCP server binary:
   ```bash
   cd /tmp/brassclaw-home-assistant-skill/mcp-server && cargo build --release
   ```
2. **Deploy Binary**:
   Copy the built binary to a permanent location under the brassclaw folder:
   ```bash
   mkdir -p ~/.brassclaw
   cp /tmp/brassclaw-home-assistant-skill/mcp-server/target/release/mcp-server ~/.brassclaw/mcp-server
   ```
3. **Register MCP Server**:
   Register the server with BrassClaw using the user's provided Home Assistant URL and Token:
   ```bash
   brassclaw mcp add homeassistant --transport stdio --command /root/.brassclaw/mcp-server --env HA_URL="<HA_URL>" --env HA_TOKEN="<HA_TOKEN>"
   ```
   *(Note: replace `/root/` with the actual home directory path if running as a different user than root).*
4. **Clean Up**:
   Remove the temporary cloned folder:
   ```bash
   rm -rf /tmp/brassclaw-home-assistant-skill
   ```

Explain each step of the progress to the user and confirm when the `homeassistant` MCP server is fully active and loaded in settings!
