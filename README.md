# IronClaw Home Assistant Extension

A WASM tool for [IronClaw](https://github.com/nearai/ironclaw) that gives the AI agent full control over a [Home Assistant](https://www.home-assistant.io/) instance via its REST API.

Capabilities include: entity state read/write, service calls, automations, scripts, scenes, MQTT publish, Modbus coil/register writes, Jinja2 template rendering, history, logbook, calendar events, persistent notifications, configuration validation, error logs, and system restart.

---

## Requirements

- Debian 12 (Bookworm) or later — commands below are for Debian/Ubuntu
- IronClaw CLI installed and working
- A running Home Assistant instance reachable from the machine
- A Home Assistant **long-lived access token**

---

## 1. Install Prerequisites

### 1a. Install Rust (if not installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

### 1b. Install IronClaw (if not installed)

Follow the official instructions at https://github.com/nearai/ironclaw.

Typically:

```bash
# Download the latest ironclaw binary for Linux x86_64
curl -L https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-linux-x86_64 \
  -o /usr/local/bin/ironclaw
chmod +x /usr/local/bin/ironclaw
ironclaw --version
```

---

## 2. Clone This Repository

```bash
git clone https://github.com/YOUR_USERNAME/ironclaw-home-assistant-skill.git
cd ironclaw-home-assistant-skill
```

---

## 3. Create a Home Assistant Long-Lived Access Token

1. Open your Home Assistant instance in a browser
2. Click your **profile name** in the left sidebar (bottom)
3. Scroll down to **Long-Lived Access Tokens**
4. Click **Create Token**, give it a name (e.g. `ironclaw`), and copy the token

Keep this token ready — you will enter it during installation.

---

## 4. Install the Extension

Run the install script from the repository root:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

The script will:

1. Install the tool from source via `ironclaw tool install ./tools-src/ha-tool` — IronClaw builds the WASM for you and auto-registers it in its **Tool Registry** (no skill required for the agent to find and use the tool).
2. Copy the **optional** skill hint to `~/.ironclaw/skills/home-assistant.SKILL.md` — this enhances the agent's context but the tool works without it.
3. Launch `ironclaw tool auth ha-tool` to securely store your HA long-lived access token (never touches the WASM sandbox — IronClaw injects it as a `Bearer` header at the host boundary).

When prompted by `ironclaw tool auth ha-tool`, paste the long-lived access token you created above.

### Expected output

```
==> Installing ha-tool from source (IronClaw will build the WASM)...
Installed successfully:
  Name: ha-tool
  WASM: ~/.ironclaw/tools/ha-tool.wasm
  Size: ~300 KB

==> Installing optional skill file (agent hint — not required)...
  Installed skill: ~/.ironclaw/skills/home-assistant.SKILL.md

==> Configuring Home Assistant access token...
  Home Assistant long-lived access token: ****
  ✓ Saved.
```

---

## 5. Verify the Installation

```bash
ironclaw tool list
```

You should see `ha-tool` in the list. For detailed info:

```bash
ironclaw tool info ha-tool
```

The skill hint (optional) lives at `~/.ironclaw/skills/home-assistant.SKILL.md`. IronClaw's Tool Registry auto-discovers the tool itself — the skill only adds extra context to the agent's system prompt.

---

## 6. Using the Extension from the IronClaw Chat Console

Start the IronClaw chat session:

```bash
ironclaw chat
```

The agent automatically activates the `home-assistant` skill when your message mentions home automation topics (lights, switches, automations, MQTT, Modbus, etc.).

**Every request must include your Home Assistant URL.** Tell the agent once at the start of the session and it will use it for all subsequent calls.

---

## Chat Usage Examples

### First message — tell the agent your HA URL

```
My Home Assistant is at http://192.168.1.100:8123
```

Or provide it inline with your first request:

```
Is my Home Assistant at http://192.168.1.100:8123 online?
```

---

### Check HA status

```
Check if my Home Assistant at http://192.168.1.100:8123 is reachable.
```

---

### List all entities (or by domain)

```
Show me all light entities in Home Assistant.
```

```
List all climate entities.
```

```
What sensors do I have?
```

```
Give me a compact list of lights, switches, and sensors in one call.
```

`get_states` accepts a single `domain_filter` (e.g. `"light"`) or an array
(e.g. `["light","switch","sensor"]`) and a `compact: true` flag that
projects each entity down to `{entity_id, state, last_changed?}` —
**use it during discovery to drastically cut token cost**, then call
`get_state` for the few entities you need full attributes for.
The response includes `matched` (post-filter count), `total` (full HA
entity count), and `cap_kind` (`"user"` or `"hard"`) when truncated.

---

### Get and set entity state

```
What is the current state of sensor.living_room_temperature?
```

```
Set light.bedroom to state 'on' with brightness 150.
```

```
Delete the manually-created state input_text.test_value.
```

`delete_state` removes a manually-created state via HA's
`DELETE /api/states/{entity_id}` endpoint — useful for cleaning up after
`set_state` experiments.

---

### Turn lights and switches on/off

```
Turn on light.living_room.
```

```
Turn off switch.garden_pump.
```

```
Set the bedroom light to 50% brightness and color temperature 3000K.
```

---

### Climate / thermostat control

```
Set climate.living_room to heat mode, target 21°C.
```

```
What is the current temperature reported by climate.bedroom?
```

---

### Automations

```
List all my automations.
```

```
Enable automation.morning_routine.
```

```
Disable automation.night_mode.
```

```
Trigger automation.welcome_home now.
```

---

### Scripts

```
List all scripts.
```

```
Run script.goodnight_routine.
```

```
Run script.set_scene with variables {"brightness": 100}.
```

---

### Scenes

```
List all scenes.
```

```
Activate scene.movie_time.
```

---

### MQTT

```
Publish MQTT message "ON" to topic home/bedroom/light/command.
```

```
Publish payload "22.5" to topic home/sensor/temp with QoS 1 and retain true.
```

---

### Modbus

```
Write true to Modbus coil at address 0, unit 1.
```

```
Write value 1500 to Modbus holding register address 10, unit 1.
```

```
Write to Modbus coil address 5, unit 2, hub "modbus_hub", value false.
```

---

### Templates

```
Render the template: {{ states('sensor.living_room_temperature') }}°C
```

```
Evaluate: {% if states('binary_sensor.door') == 'on' %}Door is open{% else %}Door is closed{% endif %}
```

`render_template` accepts optional `variables` (an object forwarded to
HA) and `max_chars` to widen the default 8 KiB output cap (hard ceiling
16 KiB). When the rendered output exceeds the cap, the response ends
with `…[truncated, N more bytes — pass `max_chars` to widen]`.

---

### History and logbook

```
Show me the state history of sensor.power_meter for the last 6 hours.
```

```
Show the logbook for automation.irrigation_morning for the last 48 hours.
```

```
Show history for light.kitchen from 2024-06-01T00:00:00Z.
```

```
Show history for light.kitchen between 2024-06-01T00:00:00Z and 2024-06-02T00:00:00Z.
```

Both `get_history` and `get_logbook` accept `start_time` and `end_time`
(ISO 8601). When neither is supplied they fall back to `hours_back`
(default 24, max 8760). **Always bound the window** to avoid
unintentionally pulling the full HA history into context.

---

### Calendar events

```
Show calendar events for calendar.family from 2024-07-01T00:00:00Z to 2024-07-07T23:59:59Z.
```

---

### Events

```
Fire event my_custom_event on the Home Assistant event bus.
```

```
Fire event notify_alarm with data {"zone": "front_door"}.
```

---

### Send a notification

```
Send a push notification to mobile_app_my_phone: "Garage door is open".
```

(Uses `call_service` with domain `notify` and the target service name from your HA notify integrations.)

---

### Persistent notifications

```
List all pending Home Assistant notifications.
```

```
Dismiss notification with ID motion_detected_123.
```

---

### Configuration and system

```
Show my Home Assistant configuration (version, location, units).
```

```
Check if the Home Assistant configuration is valid.
```

```
Show me the Home Assistant error log.
```

```
List all available services in Home Assistant.
```

```
Restart Home Assistant.
```

---

### Reloading YAML changes without restart

```
Reload Home Assistant core configuration.
```

```
Reload automations.
```

```
Reload scripts.
```

```
Reload scenes.
```

```
Reload themes.
```

```
Reload the config entry abc123def456.
```

---

## Complementary: Home Assistant MCP Server

Home Assistant ships an optional [MCP Server integration](https://www.home-assistant.io/integrations/mcp_server/) that exposes Assist-domain entities over the Model Context Protocol. IronClaw natively supports MCP clients, so you can enable both:

- **`ha-tool` (this extension)** — full REST coverage: state read/write, service calls, automations, scripts, scenes, MQTT, Modbus, templates, history, logbook, calendar, persistent notifications, reloads, error log, restart.
- **HA MCP Server (optional)** — conversational Assist-exposed entities via MCP, useful for natural-language control of the subset of entities you flag as "exposed to Assist".

They are complementary — the REST tool covers administration and maintenance the MCP server doesn't, and the MCP server provides a curated conversational surface that respects HA's Assist exposure settings.

---

## Token-Efficiency & Agent Tips

Local-LLM users should keep the agent's context tight. The tool ships
several knobs designed for this:

- **`get_states { compact: true }`** — drops the verbose `attributes`
  map and timestamps, returning only `entity_id` + `state` per entity.
  This is the single biggest token saver during discovery.
- **`get_states { domain_filter: [...] }`** — accepts an array so you
  can union-fetch multiple domains in a single round-trip instead of
  three separate calls.
- **`render_template { max_chars }`** — caps the rendered body
  (default 8 KiB, hard ceiling 16 KiB) and appends a truncation marker
  so the agent knows it can ask for more.
- **`get_history` / `get_logbook` with `start_time` + `end_time`** —
  bound the time window. Without bounds, HA can return MBs of history.
- **`shell_status`** — call once per session if you intend to use SSH
  paths so you don't waste a round-trip per shell-aware action only to
  fall back to REST.
- **Silent shell→REST fallback warning.** When `check_config` or
  `get_error_log` is called with `ssh` and the shell path fails, the
  tool logs a `Warn` and silently uses REST. `restart_ha` is the
  exception — it surfaces shell errors instead of falling back.

## Limitations

- **No real-time event subscription.** WASM tools are request/response only — there is no WebSocket event loop. Use `get_history` and `get_logbook` for time-range queries, or poll `get_state` for near-real-time monitoring.
- **No direct YAML file editing.** To apply YAML changes, edit files via HA's File Editor add-on or SSH, then run the appropriate `reload_*` action. Config Entry reloads (for integrations configured via the UI) are supported via `reload_config_entry`.

---

## Supported ha_url Formats

The `ha_url` parameter must point to a **private or local address**. Public internet IPs are blocked for security.

| Format | Example |
|---|---|
| LAN IP with port | `http://192.168.1.100:8123` |
| mDNS hostname | `http://homeassistant.local:8123` |
| Custom LAN hostname | `http://myha.lan:8123` |
| DuckDNS | `https://myha.duckdns.org` |
| Nabu Casa cloud | `https://XXXXX.ui.nabu.casa` |
| Localhost | `http://localhost:8123` |

---

## Rebuilding After Updates

If you pull new changes, rebuild and reinstall:

```bash
git pull
./scripts/install.sh
```

To build only the WASM without reinstalling:

```bash
./scripts/build.sh
```

---

## Security Notes

- The HA token is stored securely by IronClaw's secret store and never written to disk in plaintext.
- The token is automatically injected as an HTTP `Authorization: Bearer` header by the IronClaw host — it is never visible in tool parameters or chat logs.
- The `ha_url` is validated on every call and restricted to private/local address ranges to prevent token exfiltration.
- To revoke access: delete the token in Home Assistant (Profile → Long-Lived Access Tokens) and run `ironclaw tool auth ha-tool` again with a new token.

---

## Troubleshooting

**`cargo` not found**
Run `source "$HOME/.cargo/env"` or open a new terminal after installing Rust.

**`ironclaw` not found**
Ensure the binary is in your `$PATH`. Check with `which ironclaw`.

**Build fails: `error[E0463]: can't find crate for 'std'`**
The WASM target is missing. Run:
```bash
rustup target add wasm32-wasip2
```

**`ironclaw tool auth ha-tool` prompts for token but auth fails**
Ensure IronClaw is configured with a valid API key first (`ironclaw login`).

**HA API returns 401 Unauthorized**
The HA token may be expired or revoked. Create a new one in HA Profile and re-run `ironclaw tool auth ha-tool`.

**HA API returns 400 or 404**
Check that the entity ID, domain, and service name are correct. Use `get_states` or `get_services` to discover exact names.

**`ha_url` rejected as not a private address**
Your HA URL must be a local/private address. Public IPs are blocked. If you use Nabu Casa or DuckDNS, those are allowed.
