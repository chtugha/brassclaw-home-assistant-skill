---
name: home-assistant-control-search
version: 0.6.0
description: Control Home Assistant devices and search for entities
metadata:
  openclaw:
    envVars:
      - name: HA_URL
        required: true
        description: The Home Assistant URL (e.g. http://192.168.1.100:8123)
      - name: HA_TOKEN
        required: true
        description: Long-lived Access Token generated in Home Assistant Profile
activation:
  keywords:
    - home assistant
    - homeassistant
    - light
    - switch
    - thermostat
    - climate
    - temperature
    - fan
    - sensor
    - state
    - status
  patterns:
    - "turn (on|off|toggle).*(light|switch|fan|plug|outlet)"
    - "set (temperature|thermostat|climate|value|brightness)"
    - "get (status|state) of"
  tags:
    - home-automation
  max_context_tokens: 1500
---

# Home Assistant Control & Search

Use these tools to search for entities and control devices on Home Assistant:
1. **`ha_search_entities(query, domain=None)`**: Search for entities, status, or updates.
2. **`ha_control(entity_id, action, value=None)`**: Control devices (turn_on/off, toggle, set_value).

**Instructions for Common Requests**:
- **Status of Devices**: Search for the device name/type first to inspect its current state.
- **Control Devices**: Search to find the exact `entity_id` first, then call control action with that ID.

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
