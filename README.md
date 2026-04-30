# IronClaw Home Assistant Extension

Give your [IronClaw](https://github.com/nearai/ironclaw) AI agent full control over [Home Assistant](https://www.home-assistant.io/) — lights, climate, automations, sensors, MQTT, Modbus, and more — all through natural language.

---

## What You Need

Before you start, make sure you have these three things:

| # | Requirement | How to check | How to install |
|---|---|---|---|
| 1 | **Rust** | Run `cargo --version` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh -s -- -y` then **open a new terminal** |
| 2 | **IronClaw** | Run `ironclaw --version` | See [IronClaw installation](https://github.com/nearai/ironclaw) |
| 3 | **Home Assistant** | Open `http://<your-ha-ip>:8123` in a browser | [home-assistant.io/installation](https://www.home-assistant.io/installation/) |

> **Tip:** If `cargo` is not found after installing Rust, run `source "$HOME/.cargo/env"` or open a new terminal window.

---

## Installation

Three commands. The install script handles everything else.

```bash
git clone https://github.com/chtugha/ironclaw-home-assistant-skill
cd ironclaw-home-assistant-skill
./scripts/install.sh
```

The installer will walk you through 5 steps:

| Step | What happens | What you do |
|---|---|---|
| 1 | Installs build tools (`cargo-component`, WASM target) | Nothing — automatic |
| 2 | Asks for your Home Assistant URL | Type your URL (e.g. `http://192.168.1.100:8123`) |
| 3 | Compiles and installs the `ha-tool` WASM extension from source | Nothing — automatic (takes 1–2 min on first run) |
| 4 | Installs skill file, heartbeat, and routine templates | Nothing — automatic |
| 5 | Asks for your Home Assistant access token | Paste your token (see below) |

### How to Create Your Home Assistant Token

The installer will pause at Step 5 and ask you to paste a token. **Create this before you start, or have it ready when Step 5 appears.**

1. Open your Home Assistant in a browser — go directly to your **profile page**:
   `http://<your-ha-ip>:8123/profile`
2. Scroll all the way down to **Long-Lived Access Tokens**
3. Click **Create Token**
4. Name it `ironclaw` (or anything you like)
5. **Copy the token immediately** — you won't be able to see it again
6. Go back to your terminal and paste the token when prompted

> **Security:** The token is stored in IronClaw's encrypted secret store. It never appears in plaintext on disk or in chat logs.

---

## Verify It Works

After installation completes, run:

```bash
ironclaw tool list
```

You should see `ha-tool` in the output. Then start a chat:

```bash
ironclaw chat
```

Try saying:

```
Is my Home Assistant at http://192.168.1.100:8123 online?
```

(Replace with your actual URL.) The agent should respond with your HA status.

---

## What Can It Do?

Once installed, just talk to the agent naturally. Here are some examples:

| What you say | What happens |
|---|---|
| `Show me all lights.` | Lists every light entity and its state |
| `Turn on light.living_room.` | Turns on the light |
| `Set the thermostat to 21°C in heat mode.` | Calls the climate service |
| `List my automations.` | Shows all automations and their status |
| `Trigger automation.welcome_home.` | Fires the automation |
| `Run script.goodnight_routine.` | Executes the script |
| `Activate scene.movie_time.` | Activates the scene |
| `Publish "ON" to MQTT topic home/light/command.` | Sends an MQTT message |
| `Show history for sensor.temperature, last 6 hours.` | Pulls entity history |
| `Check my HA config for errors.` | Validates configuration |
| `Show the error log.` | Fetches recent error log entries |
| `Send a notification to my phone: "Garage is open".` | Pushes via notify service |
| `Reload automations.` | Reloads YAML automations without restart |

The agent figures out which `ha-tool` action to call based on what you say. You don't need to know the API — just describe what you want.

---

## Updating

```bash
cd ironclaw-home-assistant-skill
git pull
./scripts/install.sh
```

What happens on re-install:

- **HA URL** — remembered from last time. Press Enter to keep it, or type a new one.
- **ha-tool** — rebuilt and reinstalled from the latest source.
- **SKILL.md** — automatically upgraded if a newer version is available. Skipped if already up to date.
- **HEARTBEAT.md & routines.md** — **not overwritten** if they already exist (to preserve your edits). If the new version has changes, merge them manually from `heartbeat/HEARTBEAT.md` and `heartbeat/routines.md` in the repo.
- **HA token** — you will be prompted again. If your existing token still works, just paste the same one. To update it later without re-running the full installer, run `ironclaw tool auth ha-tool` directly.

---

## Accepted URL Formats

Your Home Assistant URL must point to a private/local address (public IPs are blocked for security):

| Type | Example |
|---|---|
| LAN IP (192.168.x.x) | `http://192.168.1.100:8123` |
| LAN IP (10.x.x.x) | `http://10.0.0.50:8123` |
| LAN IP (172.16–31.x.x) | `http://172.16.0.10:8123` |
| Loopback | `http://localhost:8123` or `http://127.0.0.1:8123` |
| mDNS | `http://homeassistant.local:8123` |
| Custom hostname | `http://myha.lan:8123`, `http://myha.home:8123`, `http://myha.internal:8123` |
| DuckDNS | `https://myha.duckdns.org` |
| Nabu Casa | `https://XXXXX.ui.nabu.casa` |

Using a reverse proxy with a custom domain? The installer will ask if you want to use it anyway — type `y` to confirm.

---

## Optional: Background Monitoring

The installer places two template files in `~/.ironclaw/` that enable automatic Home Assistant monitoring:

### Heartbeat (`HEARTBEAT.md`)

IronClaw reads this file on every heartbeat tick (default: every 30 minutes, configurable via `HEARTBEAT_INTERVAL_SECS`) and runs read-only health checks:

- Is HA reachable?
- Is the configuration valid?
- Are there new errors in the log?
- Are any automations stuck or unavailable?
- Are any batteries low?
- Are any updates available?

If something is wrong, the agent sends you a notification with proposed fixes. **It never makes changes without your confirmation.**

### Cron Routines (`routines.md`)

Pre-written prompts you can paste into `ironclaw chat` to create scheduled monitoring jobs:

- **Hourly health check** — status, config, notifications
- **Daily error digest** — ERROR/WARNING lines from the last 24 hours
- **Weekly update scan** — lists available HA updates every Monday
- **Stuck-automation scan** — flags unavailable automations every 6 hours
- **Battery sweep** — daily check for low-battery devices

Open `~/.ironclaw/routines.md`, copy any routine prompt, paste it into a chat session, and the agent creates the cron job for you.

---

## Local HA Instances

The IronClaw sandbox enforces HTTPS + public hostnames for WASM tool HTTP requests. For local HA instances (`http://`, `192.168.*`, `*.local`), ha-tool cannot reach the HA API. Instead, the agent uses the native `shell` tool with `curl` to call the HA REST API directly — no sandbox restrictions apply to native tools.

For public HTTPS access, run `bash scripts/setup-duckdns.sh` to configure DuckDNS + Let's Encrypt on your HA instance.

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `cargo: command not found` | Run `source "$HOME/.cargo/env"` or open a new terminal |
| `ironclaw: command not found` | Make sure IronClaw is installed and in your `$PATH` |
| Build fails with "can't find crate `std`" | Run `rustup target add wasm32-wasip2` |
| HA API returns **401 Unauthorized** | Your token is expired or revoked — create a new one in HA, then run `ironclaw tool auth ha-tool` |
| HA API returns **400** or **404** | Wrong entity ID or service name — ask the agent to `show all lights` (or sensors, etc.) first |
| URL rejected by the tool | Must be a private/local address — see [Accepted URL Formats](#accepted-url-formats) |
| Agent doesn't use `ha-tool` | Make sure `ironclaw tool list` shows `ha-tool` — re-run `./scripts/install.sh` if needed |

---

## How It Works (Technical)

- `ha-tool` is a Rust WASM component that runs inside IronClaw's sandbox
- It communicates with Home Assistant via the [REST API](https://developers.home-assistant.io/docs/api/rest/)
- Your HA token is stored in IronClaw's secret store and injected as `Authorization: Bearer` — it never enters the WASM sandbox
- The `SKILL.md` file helps IronClaw's AI know when to activate this tool based on your conversation
- All URL validation happens both in the install script (for templates) and in the Rust code (on every API call)

### Project Structure

```
ironclaw-home-assistant-skill/
├── scripts/
│   ├── install.sh          # Interactive installer (run this)
│   ├── setup-duckdns.sh    # DuckDNS + Let's Encrypt HTTPS setup
│   └── build.sh            # Standalone build script (for development)
├── tools-src/ha-tool/      # Rust WASM source code
│   ├── src/
│   │   ├── lib.rs          # Entry point, action dispatcher
│   │   ├── api.rs          # HA REST API calls
│   │   └── types.rs        # Data types and JSON schema
│   └── Cargo.toml
├── skills/SKILL.md          # AI skill hint (installed to ~/.ironclaw/skills/home-assistant/SKILL.md)
├── heartbeat/
│   ├── HEARTBEAT.md         # Background monitoring template
│   └── routines.md          # Cron routine prompts
├── dist/                    # Build output (generated by build.sh)
└── wit/tool.wit             # WASM interface definition
```

---

## Complementary: HA MCP Server

If your HA instance has the [MCP Server integration](https://www.home-assistant.io/integrations/mcp_server/) enabled, you can use both together:

- **ha-tool** — full REST API coverage: maintenance, reloads, MQTT, Modbus, templates, raw state writes, error logs, restart
- **HA MCP Server** — conversational Assist-exposed entities

They cover different parts of HA and work well side by side.

---

## License

MIT OR Apache-2.0
