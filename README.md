# BrassClaw Home Assistant Skill & MCP Server

Add full control over [Home Assistant](https://www.home-assistant.io/) — lights, climate, automations, sensors, MQTT, Modbus, and config editing — to your [BrassClaw](https://github.com/chtugha/brassclaw) AI agent through natural language.

This repository is designed specifically for BrassClaw's modern Model Context Protocol (MCP) server design and supports a fully **UI-Only, zero-terminal installation and configuration flow** for the end user.

---

## 💡 Specialized Subskills vs. Monolithic

To dramatically reduce prompt context size, increase instruction-following accuracy, and maximize the token budget for large payloads (like configuration editing and diagnostic logs), you can install individual focused subskills instead of the monolithic skill.

| Skill | Description | Supported Tools |
|---|---|---|
| **Control & Search** | Search entities and control devices. | `ha_search_entities`, `ha_control` |
| **Diagnostics** | Check health, pending updates, and alerts. | `ha_get_diagnostics` |
| **Config & Modbus** | Edit configuration.yaml and probe Modbus. | `ha_edit_config`, `ha_probe_modbus` |

---

## 🚀 Step-by-Step Installation (UI Only)

### 1. Install the Skill Manifests

You can choose to install either the monolithic skill or the split subskills.

1. Open your **BrassClaw Web UI** (usually at `http://localhost:3000` or `http://192.168.10.169:3000`).
2. Go to **Settings** > **Skills** subtab.
3. Scroll down to the **ADD CUSTOM SKILL** form.
4. Enter the name and the SKILL.md HTTPS URL of the skill you want to add:
   - **Monolithic Home Assistant**:
     - **Skill Name**: `home-assistant`
     - **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/SKILL.md`
   - **Control & Search**:
     - **Skill Name**: `home-assistant-control-search`
     - **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/control-search/SKILL.md`
   - **Diagnostics**:
     - **Skill Name**: `home-assistant-diagnostics`
     - **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/diagnostics/SKILL.md`
   - **Config & Modbus**:
     - **Skill Name**: `home-assistant-config-modbus`
     - **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/config-modbus/SKILL.md`
5. Click **Install** for each skill.

### 2. Configure via the UI

Our skill and subskills define their configuration requirements directly in their frontmatter. 
Once installed, you can configure them directly in the **BrassClaw UI** under **Settings** > **Skills**:
- **`HA_URL`**: The Home Assistant URL (e.g., `http://192.168.1.100:8123`)
- **`HA_TOKEN`**: Your long-lived access token.

### 3. Auto-Install and Register the MCP Server

Once the skill is installed and configured, the BrassClaw agent automatically knows how to build, deploy, and register the compiled Rust MCP server binary itself.

1. Go to the **Chat** tab in your BrassClaw UI.
2. Ask the agent to configure Home Assistant. For example:
   ```
   Set up Home Assistant MCP server
   ```
3. The agent will automatically:
   - Clone this repository into a temporary directory.
   - Compile the Rust `mcp-server` binary using Cargo.
   - Deploy the compiled binary to `~/.brassclaw/mcp-server`.
   - Register the `homeassistant` stdio-based MCP server into BrassClaw with the provided URL and Token env vars from the UI.
   - Clean up temporary files.
4. Once completed, the agent will confirm that the server is active. You can verify it under **Settings** > **MCP** in the UI!

---

## 🛠️ Available MCP Tools

Once installed, your agent gains access to the following 5 tools:

1. **`homeassistant_ha_search_entities(query, domain=None)`**  
   Search Home Assistant entities, sensors, or devices by name, type, area, or status.
2. **`homeassistant_ha_control(entity_id, action, value=None)`**  
   Perform control actions (e.g. `turn_on`, `turn_off`, `toggle`, `set_value` for dimming or temperature).
3. **`homeassistant_ha_get_diagnostics()`**  
   Check system health, configuration validity, software updates, and recent system logs/alerts.
4. **`homeassistant_ha_edit_config(action, file=None, old_string=None, new_string=None, offset=None, limit=None)`**  
   Safely read or patch configuration files (like `configuration.yaml`) using a token-efficient pattern.
5. **`homeassistant_ha_probe_modbus(register_type, address, host=None, port=None, unit_id=None, count=None)`**  
   Directly probe Modbus TCP registers or coils for advanced integrations.

---

## 💬 Example Natural Language Commands

Once everything is configured, you can talk to BrassClaw naturally:

* **Devices**: "Turn on the kitchen overhead lights." or "Dim the living room to 40%."
* **Climate**: "What is the temperature in the bedroom?" or "Set the thermostat to 21 degrees."
* **System**: "Are there any software updates available for my Home Assistant?" or "Check my configuration for errors."
* **Modbus / Advanced**: "Probe holding register 100 on Modbus."

---

## ⚙️ Requirements & Technical Details

* **Rust Runtime**: The host running BrassClaw must have Rust installed so the agent can compile the high-performance MCP server binary from source on first setup.
* **Network Permissions**: Ensure `HTTP_ALLOW_LOCALHOST=true` is set in the BrassClaw daemon's environment so that local Home Assistant private IPs can be reached.

---

## 📄 License

This project is licensed under the MIT OR Apache-2.0 License.
