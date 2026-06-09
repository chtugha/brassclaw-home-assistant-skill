# BrassClaw Home Assistant Skill

Add full control over [Home Assistant](https://www.home-assistant.io/) — lights, climate, automations, sensors, and more — to your [BrassClaw](https://github.com/chtugha/brassclaw) AI agent through natural language.

This skill uses BrassClaw's native HTTP tool with automatic credential injection. No external MCP servers or compilation required.

---

## 💡 Specialized Subskills vs. Monolithic

To reduce prompt context size and improve performance, you can install individual focused subskills instead of the monolithic skill.

| Skill | Description | Context Tokens |
|---|---|---|
| **Monolithic** | All features in one skill | 3000 |
| **Control & Search** | Search entities and control devices | 1500 |
| **Diagnostics** | Check health, updates, and logs | 2500 |
| **Config & Modbus** | Edit config and probe Modbus | 3000 |

---

## 🚀 Installation

### 1. Install the Skill

1. Open your **BrassClaw Web UI** (usually at `http://localhost:3000`)
2. Go to **Settings** > **Skills**
3. Scroll to **ADD CUSTOM SKILL**
4. Choose one of the following:

#### Monolithic Skill (All Features)
- **Skill Name**: `home-assistant`
- **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skill.md`

#### Or Install Subskills Individually

**Control & Search**:
- **Skill Name**: `home-assistant-control-search`
- **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/control-search/skill.md`

**Diagnostics**:
- **Skill Name**: `home-assistant-diagnostics`
- **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/diagnostics/skill.md`

**Config & Modbus**:
- **Skill Name**: `home-assistant-config-modbus`
- **HTTPS URL**: `https://raw.githubusercontent.com/chtugha/brassclaw-home-assistant-skill/main/skills/config-modbus/skill.md`

5. Click **Install**

### 2. Configure Credentials

After installation, configure the skill in **Settings** > **Skills**:

- **`ha_url`**: Your Home Assistant URL (e.g., `http://192.168.1.100:8123`)
- **`ha_token`**: Long-lived access token from Home Assistant
  - Create at: Profile → Security → Long-Lived Access Tokens

Credentials are automatically injected into HTTP requests by BrassClaw's credential system.

---

## 🛠️ Available Operations

### Control & Search
- Search for entities by name, domain, or area
- Turn devices on/off
- Toggle switches and lights
- Set brightness, temperature, and other values
- Check device status and states

### Diagnostics
- Check system health and configuration
- Find pending software updates
- View recent error logs
- Monitor integration status
- Check for unavailable entities

### Config & Modbus
- Validate configuration files
- Edit configuration.yaml (requires SSH)
- Probe Modbus TCP registers
- Read/write Modbus coils and holding registers

---

## 💬 Example Commands

Once configured, talk to BrassClaw naturally:

**Device Control**:
- "Turn on the kitchen lights"
- "Dim the living room to 40%"
- "Set the bedroom thermostat to 21 degrees"
- "Toggle the garage door"

**Status Checks**:
- "What's the temperature in the bedroom?"
- "Is the front door locked?"
- "Show me all unavailable devices"

**System Management**:
- "Check for Home Assistant updates"
- "Show me recent errors"
- "What's the system health status?"

**Advanced**:
- "Probe Modbus register 100"
- "Add a new sensor to my configuration"

---

## 🔧 How It Works

This skill uses BrassClaw's native capabilities:

1. **HTTP Tool**: Makes REST API calls to Home Assistant
2. **Credential Injection**: Automatically adds authentication headers
3. **Shell Tool**: For SSH access (config editing, Modbus probing)

No external MCP servers, no compilation, no dependencies. Just install and configure.

---

## 📋 Requirements

- **BrassClaw**: Version 2.0+ with native HTTP tool support
- **Home Assistant**: Any recent version with REST API enabled
- **Network Access**: BrassClaw must be able to reach your Home Assistant instance
- **SSH Access** (optional): Required for configuration editing and Modbus operations

---

## 🔒 Security Notes

- Store your `ha_token` securely in BrassClaw's credential system
- Use HTTPS for Home Assistant if possible
- Limit token permissions to necessary scopes
- For SSH operations, use key-based authentication
- Be cautious with Modbus write operations

---

## 🐛 Troubleshooting

**Skill not activating?**
- Check that keywords match your request
- Verify credentials are configured correctly
- Ensure Home Assistant is reachable from BrassClaw

**API errors?**
- Verify your `ha_url` is correct (include port if needed)
- Check that your `ha_token` is valid and not expired
- Ensure Home Assistant REST API is enabled

**Configuration editing not working?**
- SSH access is required for config file editing
- Alternatively, use the File Editor add-on
- Check file permissions on Home Assistant host

---

## 📚 API Reference

This skill uses the [Home Assistant REST API](https://developers.home-assistant.io/docs/api/rest/):

- `/api/states` - Get all entity states
- `/api/states/{entity_id}` - Get specific entity
- `/api/services/{domain}/{service}` - Call a service
- `/api/config` - Get system configuration
- `/api/error/all` - Get error logs

---

## 🤝 Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Test your changes thoroughly
4. Submit a pull request

---

## 📄 License

MIT OR Apache-2.0

---

## 🔗 Links

- [BrassClaw](https://github.com/chtugha/brassclaw)
- [Home Assistant](https://www.home-assistant.io/)
- [Home Assistant REST API Docs](https://developers.home-assistant.io/docs/api/rest/)
