---
name: home-assistant-diagnostics
version: 0.6.0
description: Check Home Assistant system health, software updates, and logs/alerts
metadata:
  openclaw:
    requires:
      env:
        - HA_URL
        - HA_TOKEN
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
    - diagnostics
    - logs
    - errors
    - updates
    - system health
    - alerts
  patterns:
    - "check (diagnostics|health|logs|errors|updates) of (home assistant|homeassistant)"
    - "is there any update"
  tags:
    - home-automation
  max_context_tokens: 2500
---

# Home Assistant Diagnostics

Use this tool to check system health, logs, and updates on Home Assistant:
1. **`homeassistant_ha_get_diagnostics()`**: Check health, logs, or updates.

**Instructions for Common Requests**:
- **Updates / System Health**: Call `homeassistant_ha_get_diagnostics()`. This checks for general health, active integrations, pending software updates, and recent errors or alerts.

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
