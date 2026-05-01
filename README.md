# IronClaw Home Assistant Extension

Give your [IronClaw](https://github.com/nearai/ironclaw) AI agent full control over [Home Assistant](https://www.home-assistant.io/) — lights, climate, automations, sensors, MQTT, Modbus, and more — all through natural language.

---

## Choose Your Installer

This extension comes in two variants. **Install one, not both** — the installers handle mutual exclusion automatically.

| | Local | Remote |
|---|---|---|
| **For** | HA on your LAN (`http://`, `192.168.*`, `*.local`) | HA via public HTTPS (Nabu Casa, DuckDNS) |
| **How it works** | Agent uses built-in `http` or `shell` tool | Agent uses `ha-tool` WASM component |
| **Install command** | `./local/scripts/install.sh` | `./scripts/install.sh` |
| **Requires Rust** | No | Yes |
| **Install time** | < 1 second | 1–2 minutes (WASM compile) |
| **Extras** | Dual-mode (http/shell), works in routines | Structured responses, input validation, compact mode |

> **Most users should install Local.** It works with any HA instance, requires no build tools, and is more reliable in routines and server mode.

---

## What You Need

| # | Requirement | How to check | How to install |
|---|---|---|---|
| 1 | **IronClaw** | Run `ironclaw --version` | See [IronClaw installation](https://github.com/nearai/ironclaw) |
| 2 | **Home Assistant** | Open `http://<your-ha-ip>:8123` in a browser | [home-assistant.io/installation](https://www.home-assistant.io/installation/) |
| 3 | **Rust** (remote only) | Run `cargo --version` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh -s -- -y` then **open a new terminal** |

---

## Installation — Local (recommended)

Two commands. No build step.

```bash
git clone https://github.com/chtugha/ironclaw-home-assistant-skill
cd ironclaw-home-assistant-skill
./local/scripts/install.sh
```

The installer asks for your HA URL and access token, copies files to `~/.ironclaw/`, and guides you through setting `HTTP_ALLOW_LOCALHOST=true` for the most reliable operation.

> **Important:** After installation, set `HTTP_ALLOW_LOCALHOST=true` in your IronClaw environment and restart. This enables the built-in `http` tool to reach your local HA — it works in all contexts (CLI, server, routines, jobs) without depending on the `shell` tool.

## Installation — Remote (public HTTPS only)

```bash
git clone https://github.com/chtugha/ironclaw-home-assistant-skill
cd ironclaw-home-assistant-skill
./scripts/install.sh
```

The installer will walk you through 5 steps:

| Step | What happens | What you do |
|---|---|---|
| 1 | Installs build tools (`cargo-component`, WASM target) | Nothing — automatic |
| 2 | Asks for your Home Assistant URL | Type your HTTPS URL (e.g. `https://myha.duckdns.org`) |
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

After installation completes, start a chat:

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

The agent figures out which API calls to make based on what you say. You don't need to know the API — just describe what you want.

---

## Updating

```bash
cd ironclaw-home-assistant-skill
git pull
./local/scripts/install.sh    # or ./scripts/install.sh for remote
```

What happens on re-install:

- **HA URL** — remembered from last time. Press Enter to keep it, or type a new one.
- **SKILL.md** — automatically upgraded if a newer version is available. Skipped if already up to date.
- **HEARTBEAT.md & routines.md** — **not overwritten** if they already exist (to preserve your edits). If the new version has changes, merge them manually.
- **ha-tool** (remote only) — rebuilt and reinstalled from the latest source.
- **HA token** — re-prompted. Paste the same token, or create a new one.

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

Pre-written prompts you can paste into `ironclaw chat` to create scheduled monitoring jobs. All routines support dual-mode (http/shell) and will use whichever tool is available:

- **Hourly health check** — status, config, notifications
- **Daily error digest** — ERROR/WARNING lines from the last 24 hours
- **Weekly update scan** — lists available HA updates every Monday
- **Stuck-automation scan** — flags unavailable automations every 6 hours
- **Battery sweep** — daily check for low-battery devices

Open `~/.ironclaw/routines.md`, copy any routine prompt, paste it into a chat session, and the agent creates the cron job for you.

---

## Local vs Remote — When to Use Which

**Local** (recommended): Use `./local/scripts/install.sh` for any HA on your LAN. The agent uses IronClaw's built-in `http` tool (preferred) or `shell` + `curl` (fallback) to call the HA REST API. No build tools, no sandbox restrictions, works with `http://`. Set `HTTP_ALLOW_LOCALHOST=true` for the most reliable operation.

**Remote**: Use `./scripts/install.sh` only when your HA is accessible via public HTTPS (Nabu Casa Cloud or DuckDNS + Let's Encrypt). Provides structured JSON responses, input validation, and compact entity output via the `ha-tool` WASM component. To set up DuckDNS, run `bash scripts/setup-duckdns.sh`.

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `Tool 'shell' failed: Tool shell not found` | Set `HTTP_ALLOW_LOCALHOST=true` in your IronClaw environment and restart. The agent will use the built-in `http` tool instead. |
| `only https URLs are allowed` or `private or local IPs are not allowed` | Set `HTTP_ALLOW_LOCALHOST=true` in your IronClaw environment and restart. |
| `ironclaw: command not found` | Make sure IronClaw is installed and in your `$PATH` |
| `cargo: command not found` (remote only) | Run `source "$HOME/.cargo/env"` or open a new terminal |
| Build fails with "can't find crate `std`" (remote only) | Run `rustup target add wasm32-wasip2` |
| HA API returns **401 Unauthorized** | Your token is expired or revoked — create a new one in HA |
| HA API returns **400** or **404** | Wrong entity ID or service name — ask the agent to `show all lights` first |
| Agent doesn't use HA skill | Run `ironclaw skills list` and check for `home-assistant-local` or `home-assistant` |
| Both skills showing up | Re-run the installer for the one you want — it removes the other automatically |

---

## How It Works (Technical)

- **Local extension** uses IronClaw's built-in `http` tool (preferred, always available with `HTTP_ALLOW_LOCALHOST=true`) or `shell` + `curl` (fallback). The agent tries `http` first and falls back to `shell` automatically.
- **Remote extension** uses `ha-tool`, a Rust WASM component that runs inside IronClaw's sandbox
- Both communicate with Home Assistant via the [REST API](https://developers.home-assistant.io/docs/api/rest/)
- Your HA token is stored locally in `~/.ironclaw/.ha_token`
- The `SKILL.md` file helps IronClaw's AI know when to activate this tool based on your conversation
- All URL validation happens both in the install script (for templates) and in the Rust code (on every API call)

### Project Structure

```
ironclaw-home-assistant-skill/
├── local/                     # Local extension (http tool + shell fallback)
│   ├── scripts/install.sh     #   Local installer (run this for local HA)
│   ├── skills/SKILL.md        #   Skill file (dual-mode: http/shell)
│   └── heartbeat/
│       ├── HEARTBEAT.md       #   Heartbeat template (dual-mode)
│       └── routines.md        #   Cron routine prompts (dual-mode)
├── scripts/
│   ├── install.sh             # Remote installer (run this for public HTTPS HA)
│   ├── setup-duckdns.sh       # DuckDNS + Let's Encrypt HTTPS setup
│   └── build.sh               # Standalone build script (for development)
├── tools-src/ha-tool/         # Rust WASM source code (remote only)
│   ├── src/
│   │   ├── lib.rs             #   Entry point, action dispatcher
│   │   ├── api.rs             #   HA REST API calls
│   │   └── types.rs           #   Data types and JSON schema
│   └── Cargo.toml
├── skills/SKILL.md            # Remote skill file (ha-tool actions)
├── heartbeat/
│   ├── HEARTBEAT.md           # Remote heartbeat template (ha-tool)
│   └── routines.md            # Remote cron routine prompts (ha-tool)
├── dist/                      # Build output (generated by build.sh)
└── wit/tool.wit               # WASM interface definition
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
