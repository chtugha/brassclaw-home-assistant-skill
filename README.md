# BrassClaw Home Assistant Extension

Give your [BrassClaw](https://github.com/nearai/brassclaw) AI agent full control over [Home Assistant](https://www.home-assistant.io/) — lights, climate, automations, sensors, MQTT, Modbus, and more — all through natural language.

---

## Table of Contents

1. [What This Does](#what-this-does)
2. [Before You Begin — Checklist](#before-you-begin--checklist)
3. [Step 1 — Which Installer Do I Need?](#step-1--which-installer-do-i-need)
4. [Step 2A — Local Installer (Recommended)](#step-2a--local-installer-recommended)
5. [Step 2B — Remote Installer (Public HTTPS only)](#step-2b--remote-installer-public-https-only)
6. [Step 3 — Verify It Works](#step-3--verify-it-works)
7. [What Can It Do?](#what-can-it-do)
8. [Background Monitoring](#background-monitoring)
9. [Updating the Extension](#updating-the-extension)
10. [Troubleshooting](#troubleshooting)
11. [Optional: DuckDNS + HTTPS Setup](#optional-duckdns--https-setup)
12. [How It Works (Technical)](#how-it-works-technical)

---

## What This Does

This extension teaches BrassClaw how to control Home Assistant. Once installed, you just talk to BrassClaw normally:

```
Turn on the living room lights.
Set the thermostat to 21 degrees.
Show me any sensors with problems.
Check the Home Assistant config for errors.
```

BrassClaw figures out what API calls to make. You don't need to know the API.

---

## Before You Begin — Checklist

Work through this checklist **before** running the installer. The installer will fail immediately if these are not in place.

### ✅ 1. BrassClaw is installed

Open a terminal and run:
```bash
brassclaw --version
```

If you see a version number, you're good. If you see `command not found`, install BrassClaw first:  
→ https://github.com/nearai/brassclaw

---

### ✅ 2. Home Assistant is running and reachable

Open a browser and go to your HA address. It should show the Home Assistant login page or dashboard.

You need to know **your HA URL** — you will be asked for it during installation.  
Examples of valid URLs:

| Situation | Your URL looks like |
|---|---|
| HA on your home network | `http://192.168.1.100:8123` |
| HA on your home network | `http://homeassistant.local:8123` |
| HA running on the same machine | `http://localhost:8123` |
| HA via DuckDNS | `https://myha.duckdns.org` |
| HA via Nabu Casa Cloud | `https://XXXXXX.ui.nabu.casa` |

> **Don't know your HA IP?** Open Home Assistant in a browser. Look at the address bar — that is your URL. Copy it exactly (including the port number if it shows one).

---

### ✅ 3. You have a Home Assistant access token

The extension needs a **long-lived access token** to call the HA API. Here is how to create one:

1. Open Home Assistant in your browser
2. Click your **name** in the bottom-left sidebar (or go to `/profile` directly)
3. Scroll all the way down to the section called **Long-Lived Access Tokens**
4. Click the **Create Token** button
5. Give it a name — for example: `brassclaw`
6. Click **OK**
7. **Copy the token now** — it will never be shown again

> Keep this token in your clipboard or a text file until the installer asks for it.

---

### ✅ 4. Rust is installed (remote installer only)

If you are using the **remote installer** (for Nabu Casa or DuckDNS), you also need Rust.

Check if Rust is installed:
```bash
cargo --version
```

If you see a version number, you're good. If not, install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

After it finishes, **open a new terminal window** (or run `source "$HOME/.cargo/env"`) before continuing.

---

## Step 1 — Which Installer Do I Need?

Answer this one question: **How do you access Home Assistant?**

```
My HA URL starts with http://
→ Use the LOCAL installer (Step 2A)

My HA URL starts with https://
→ Use the REMOTE installer (Step 2B)

I'm not sure
→ Use the LOCAL installer (Step 2A) — it works for both
```

| | **Local** | **Remote** |
|---|---|---|
| HA URL | `http://...` (LAN, localhost) | `https://...` (Nabu Casa, DuckDNS) |
| Needs Rust? | **No** | Yes (for WASM compilation) |
| Install time | Under 5 seconds | 1–5 minutes first run |
| Works in routines/jobs? | **Yes** (with `HTTP_ALLOW_LOCALHOST=true`) | Yes |
| Recommended? | **Yes — for most users** | Only for public HTTPS HA |

> **When in doubt: use the Local installer.** It works with any HA instance and has fewer moving parts.

---

## Step 2A — Local Installer (Recommended)

### 2A.1 — Get the code

Open a terminal in /root and run these two commands one at a time:

```bash
git clone https://github.com/chtugha/brassclaw-home-assistant-skill
cd brassclaw-home-assistant-skill
```

You should now be inside the `brassclaw-home-assistant-skill` folder.

---

### 2A.2 — Run the installer

```bash
./local/scripts/install.sh
```

The installer will guide you through 4 steps. Here is exactly what to expect:

---

**[1/4] Configuring Home Assistant URL**

The installer asks for your HA URL.

```
Home Assistant URL: 
```

Type your HA URL and press Enter. For example:
```
http://192.168.1.100:8123
```

If the URL passes validation, it is saved automatically. If the installer says the URL looks unusual, you can type `y` to accept it anyway.


```
brassclaw mcp add homeassistant \
  --transport stdio \
  --command /root/ironclaw-home-assistant-skill/mcp-server/target/release/mcp-server \
  --env HA_URL=http://localhost:8123 \
  --env HA_TOKEN=YOUR_TOKEN_HERE
```

---

**[2/4] Installing skill, heartbeat, and routine files**

No input needed. The installer copies configuration files to `~/.brassclaw/`.

---

**[3/4] Configuring Home Assistant access token**

The installer asks for the token you created in the checklist above.

```
HA Token: 
```

Paste your token and press Enter. **The token is not echoed** — the cursor will not move. That is normal. Press Enter once and wait.

The token is saved to `~/.brassclaw/.ha_token`, `~/.brassclaw/HEARTBEAT.md`, and `~/.brassclaw/routines.md`, all with restricted permissions (`chmod 600`). HEARTBEAT.md and routines.md need the token so the agent can make API calls during scheduled heartbeat ticks and cron routines, where it cannot read files interactively.

---

**[4/4] Checking environment configuration**

The installer checks whether `HTTP_ALLOW_LOCALHOST=true` is set in your environment.

If it is **not** set, you will see instructions. Follow them now — this is important for the extension to work reliably in all contexts.

---

### 2A.3 — Set HTTP_ALLOW_LOCALHOST=true (Required)

This environment variable lets BrassClaw's built-in `http` tool call local addresses and private IPs. Without it, the agent can only use the `shell` tool — which is not available in scheduled jobs and server mode.

**Choose the method that matches how you run BrassClaw:**

**Option A — CLI (run manually in your terminal)**

Add this line to your shell profile (`~/.zshrc`, `~/.bashrc`, or `~/.bash_profile`):
```bash
export HTTP_ALLOW_LOCALHOST=true
```
Then run:
```bash
source ~/.zshrc   # or ~/.bashrc — whichever you edited
```

**Option B — systemd service (runs as a background service)**

Edit your BrassClaw service file (`/etc/systemd/system/brassclaw.service` or similar):
```ini
[Service]
Environment=HTTP_ALLOW_LOCALHOST=true
```
Then reload and restart:
```bash
sudo systemctl daemon-reload
sudo systemctl restart brassclaw
```

**Option C — Docker**

Add to your `docker run` command:
```
-e HTTP_ALLOW_LOCALHOST=true
```

Or add to your `docker-compose.yml`:
```yaml
environment:
  - HTTP_ALLOW_LOCALHOST=true
```

**Option D — .env file**

Add to the `.env` file in your BrassClaw working directory:
```
HTTP_ALLOW_LOCALHOST=true
```

After setting the variable, **restart BrassClaw**.

---

### 2A.4 — Confirm installation

At the end, the installer prints a summary:

```
✓ Local HA extension installed!

Configuration saved:
  HA URL:     ~/.brassclaw/.ha_url
  HA Token:   ~/.brassclaw/.ha_token
  Skill:      ~/.brassclaw/skills/home-assistant-local/SKILL.md
  Heartbeat:  ~/.brassclaw/HEARTBEAT.md
  Routines:   ~/.brassclaw/routines.md
```

All three lines (Skill, Heartbeat, Routines) should show file paths — not "not installed". If any do, re-run the installer.

**You are done with the local installer. Skip to [Step 3](#step-3--verify-it-works).**

---

## Step 2B — Remote Installer (Public HTTPS only)

Use this only if your HA is accessible via `https://` with a public hostname (Nabu Casa or DuckDNS). **Do not use this for local `http://` HA instances** — the sandbox cannot reach private IPs.

### 2B.1 — Get the code

```bash
git clone https://github.com/chtugha/brassclaw-home-assistant-skill
cd brassclaw-home-assistant-skill
```

---

### 2B.2 — Run the installer

```bash
./scripts/install.sh
```

The installer walks you through 5 steps. Here is exactly what to expect:

---

**[1/5] Checking build dependencies**

No input needed. The installer installs `cargo-component` and the WASM build target automatically.

---

**[2/5] Configuring Home Assistant URL**

```
Home Assistant URL: 
```

Type your public HTTPS URL and press Enter. For example:
```
https://myha.duckdns.org
```
or
```
https://XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.ui.nabu.casa
```

> **If your URL starts with `http://`**, the installer will warn you that a local address was detected and suggest using the local installer. You can still continue with `y` if you want, but the WASM sandbox cannot reach private IPs — use the local installer instead.

---

**[3/5] Installing ha-tool from source**

No input needed. BrassClaw compiles the `ha-tool` WASM extension from source. This takes **1–5 minutes** the first time. Subsequent installs are faster due to build caching.

You should see Rust compilation output ending with:
```
Finished `release` profile [optimized] target(s) in ...
```

---

**[4/5] Installing skill and heartbeat files**

No input needed. Files are copied to `~/.brassclaw/`.

---

**[5/5] Configuring Home Assistant access token**

```
HA Token: 
```

Paste your token and press Enter. The installer calls `brassclaw tool auth ha-tool` to store it in BrassClaw's encrypted secret store.

BrassClaw may show its own prompt — follow the on-screen instructions and paste the token again when asked.

> If your token still works and you are re-installing, run `brassclaw tool auth ha-tool` directly to update it without re-running the full installer.

---

### 2B.3 — Confirm installation

```
✓ ha-tool installed successfully!

Configuration saved:
  HA URL:     ~/.brassclaw/.ha_url
  Skill:      ~/.brassclaw/skills/home-assistant/SKILL.md
  Heartbeat:  ~/.brassclaw/HEARTBEAT.md
  Routines:   ~/.brassclaw/routines.md
```

---

## Step 3 — Verify It Works

### 3.1 — Start a chat

```bash
brassclaw chat
```

Wait for the prompt to appear.

---

### 3.2 — Ask about your Home Assistant

Type this (replace the URL with your actual HA address):

```
Is my Home Assistant online?
```

The agent should respond with something like:

```
Your Home Assistant is online. It's running version 2024.x.x on ...
```

If you see an error instead, go to [Troubleshooting](#troubleshooting).

---

### 3.3 — Try a real command

```
List all my lights.
```

The agent should return a list of your light entities.

---

## What Can It Do?

Once the extension is installed, just talk to the agent naturally. No API knowledge required.

### Lights, Switches, and Covers

```
Turn on the living room lights.
Dim the bedroom light to 30%.
Turn off all lights in the house.
Open the garage door.
Close the living room blinds.
```

### Climate and Temperature

```
Set the thermostat to 21 degrees in heat mode.
What is the current temperature in the bedroom?
Turn off the air conditioning.
```

### Automations, Scripts, and Scenes

```
List all my automations.
Enable automation.welcome_home.
Trigger automation.morning_routine.
Run script.goodnight.
Activate scene.movie_time.
```

### Sensors and Monitoring

```
Which sensors are unavailable?
Are any battery levels below 20%?
Show the history for sensor.temperature for the last 6 hours.
Show me today's calendar events.
```

### MQTT and Modbus

```
Publish "ON" to MQTT topic home/plug/command.
Write value 1 to Modbus holding register 100 on unit 1.
```

### System and Maintenance

```
Check my Home Assistant config for errors.
Show the last 50 lines of the error log.
Reload automations.
Reload the MQTT integration without restarting.
Send a notification to my phone: "Garage is open".
```

---

## Background Monitoring

The installer places two files in `~/.brassclaw/` that enable automatic monitoring of your Home Assistant. These are optional — the extension works without them.

### Heartbeat Monitoring (`~/.brassclaw/HEARTBEAT.md`)

BrassClaw reads `HEARTBEAT.md` on every heartbeat tick (default every 30 minutes) and automatically runs these read-only checks:

- **Reachability** — is HA responding?
- **Config validation** — is the YAML configuration valid?
- **Error log** — are there new errors since the last tick?
- **Stuck automations** — any automations unavailable or not triggered in 30 days?
- **Problem sensors** — battery_low, connectivity lost, sensor unavailable?

If anything is wrong, the agent sends you a notification with proposed fixes. **It never makes changes without your explicit confirmation.**

To enable heartbeat monitoring, configure a heartbeat schedule in BrassClaw. See the BrassClaw documentation for `HEARTBEAT_INTERVAL_SECS`.

---

### Scheduled Routines (`~/.brassclaw/routines.md`)

`routines.md` contains ready-to-use prompts for creating scheduled monitoring jobs. To set up a routine:

1. Open `~/.brassclaw/routines.md` in any text editor
2. Copy one of the routine prompts (the text inside the triple backtick blocks)
3. Start a chat: `brassclaw chat`
4. Paste the prompt and send it
5. The agent creates the cron job for you

**Available routine prompts:**

| Routine | Schedule | What it does |
|---|---|---|
| `ha-hourly-health` | Every hour | Status check, config validation, notifications |
| `ha-daily-errors` | Daily at 08:00 | Error/warning digest from the last 24 hours |
| `ha-weekly-updates` | Monday at 09:00 | Lists available HA updates |
| `ha-automation-health` | Every 6 hours | Flags unavailable or stuck automations |
| `ha-battery-check` | Daily at 18:00 | Lists devices with battery below 20% |

All routines are **read-only by design** — they never change anything without your confirmation in chat.

---

## Updating the Extension

To update to the latest version:

```bash
cd brassclaw-home-assistant-skill
git pull
./local/scripts/install.sh    # or ./scripts/install.sh for remote
```

What happens when you re-run the installer:

| Item | Behavior |
|---|---|
| **HA URL** | Pre-filled from last time. Press Enter to keep it, or type a new one. |
| **SKILL.md** | Auto-updated if the new version is newer. Skipped if already up to date. |
| **HEARTBEAT.md** | **Not overwritten** if it already exists and is from the same variant. Your edits are preserved. |
| **routines.md** | Same as HEARTBEAT.md — preserved if already exists. |
| **HA token** | Re-prompted. Paste the same token to keep it. |
| **ha-tool** (remote) | Rebuilt and reinstalled from latest source. |

> **Switched from local to remote (or vice versa)?** The installer detects this automatically and replaces the wrong-variant heartbeat/routines files. Your HA URL is remembered across both installers.

---

## Troubleshooting

### "Tool 'shell' failed: Tool shell not found"

The agent tried to use the `shell` tool but it is not available in the current context (it is only registered at BrassClaw startup with `allow_local_tools = true`, and not available in scheduled jobs or server mode).

**Fix:** Set `HTTP_ALLOW_LOCALHOST=true` in your BrassClaw environment and restart. The agent will use the built-in `http` tool instead, which works everywhere.

See [2A.3 — Set HTTP_ALLOW_LOCALHOST=true](#2a3--set-http_allow_localhosttrue-required) for how to set it.

---

### "only https URLs are allowed" or "private or local IPs are not allowed"

The `http` tool is available but `HTTP_ALLOW_LOCALHOST=true` is not set (or BrassClaw was not restarted after setting it).

**Fix:** Set `HTTP_ALLOW_LOCALHOST=true` and **restart BrassClaw**. Just setting the variable without restarting does nothing.

---

### `brassclaw: command not found`

BrassClaw is not installed or not in your PATH.

**Fix:** Install BrassClaw from https://github.com/nearai/brassclaw and make sure its binary is in your PATH.

---

### `cargo: command not found` (remote installer only)

Rust is not installed, or the environment is not set up in the current terminal.

**Fix:**
```bash
source "$HOME/.cargo/env"
```
Or open a new terminal window.

---

### Build fails with "can't find crate `std`" (remote installer only)

The WASM target is missing.

**Fix:**
```bash
rustup target add wasm32-wasip2
```

---

### HA API returns 401 Unauthorized

Your access token is expired, revoked, or incorrect.

**Fix:** Create a new long-lived access token in HA (see the [checklist](#-3-you-have-a-home-assistant-access-token)) and re-run the installer to update it.

---

### HA API returns 404 Not Found

The entity ID or service name does not exist on your HA instance.

**Fix:** First ask the agent to discover what's available:
```
Show me all my automations.
List all lights.
```

---

### The agent ignores Home Assistant and does something else

The skill file is not loaded.

**Fix:** Check that the skill is installed:
```bash
brassclaw skills list
```

You should see `home-assistant-local` (local extension) or `home-assistant` (remote extension). If neither appears, re-run the installer.

---

### Both "home-assistant" and "home-assistant-local" appear in `brassclaw skills list`

Both installers have been run. Having both wastes token budget.

**Fix:** Re-run whichever installer you want to keep. It automatically removes the other.

---

### The heartbeat runs but the agent cannot reach HA

`HTTP_ALLOW_LOCALHOST=true` is set in your CLI shell but not in the service/daemon that runs the heartbeat.

**Fix:** Make sure the environment variable is set where BrassClaw is actually running (the service, not just your terminal session). See [2A.3](#2a3--set-http_allow_localhosttrue-required).

---

## Optional: DuckDNS + HTTPS Setup

If your HA is on your LAN but you want to access it via HTTPS (for example to use the remote extension, or for external access), you can set up DuckDNS + Let's Encrypt automatically.

**Requirements:**
- Home Assistant OS (Supervisor required)
- A free DuckDNS account: https://www.duckdns.org
- A long-lived HA access token
- Port 443 forwarded on your router to your HA host

**Run the setup script:**
```bash
./scripts/setup-duckdns.sh
```

The script will:
1. Connect to your HA Supervisor API
2. Install the DuckDNS add-on from the HA store
3. Configure it with your DuckDNS token and domain
4. Enable Let's Encrypt certificates
5. Start the add-on

After the script finishes, you still need to:
1. Add to `configuration.yaml`:
   ```yaml
   http:
     ssl_certificate: /ssl/fullchain.pem
     ssl_key: /ssl/privkey.pem
   ```
2. Restart Home Assistant
3. Re-run `./scripts/install.sh` with your new `https://yourdomain.duckdns.org` URL

---

## How It Works (Technical)

### Two Variants

**Local extension** (`local/`)
- Uses BrassClaw's built-in `http` tool (preferred when `HTTP_ALLOW_LOCALHOST=true`) or `shell + curl` (fallback when `allow_local_tools = true`)
- The agent tries `http` first; if it gets an SSRF/HTTPS error, it falls back to `shell`
- No compilation required — installs in under 5 seconds
- Works with `http://` and private IPs

**Remote extension** (`tools-src/ha-tool/`)
- A Rust WASM component compiled to `wasm32-wasip2` and loaded into BrassClaw's sandbox
- The sandbox enforces HTTPS and blocks private IPs — only Nabu Casa and DuckDNS domains are permitted
- Provides structured JSON responses, strong input validation, compact entity projection, and SSRF protection
- The HA token is managed by BrassClaw's secret store and injected automatically — the WASM component never sees the raw token value

### Files Installed

| File | Location | Purpose |
|---|---|---|
| `SKILL.md` | `~/.brassclaw/skills/home-assistant/SKILL.md` | Tells BrassClaw's AI when and how to use the extension |
| `HEARTBEAT.md` | `~/.brassclaw/HEARTBEAT.md` | Read-only health check instructions for heartbeat ticks; contains injected URL + token (`chmod 600`, local only) |
| `routines.md` | `~/.brassclaw/routines.md` | Ready-to-paste prompts for cron monitoring jobs; contains injected URL + token (`chmod 600`, local only) |
| `.ha_url` | `~/.brassclaw/.ha_url` | Saved HA URL (reused across reinstalls) |
| `.ha_token` | `~/.brassclaw/.ha_token` | Saved HA token (local extension only, `chmod 600`) |

### Project Structure

```
brassclaw-home-assistant-skill/
├── local/                     # Local extension (http tool + shell fallback)
│   ├── scripts/install.sh     #   Installer — run this for local http:// HA
│   └── heartbeat/
│       ├── HEARTBEAT.md       #   Heartbeat template (dual-mode)
│       └── routines.md        #   Cron routine prompts (dual-mode)
│
├── scripts/
│   ├── install.sh             # Remote installer — run this for https:// HA
│   ├── setup-duckdns.sh       # Automated DuckDNS + Let's Encrypt setup
│   └── build.sh               # Standalone WASM build (for development)
│
├── tools-src/ha-tool/         # Rust source for the remote WASM tool
│   └── src/
│       ├── lib.rs             #   Entry point + action dispatcher
│       ├── api.rs             #   HA REST API calls + validation
│       └── types.rs           #   JSON schema + data types
│
├── SKILL.md                   # Skill hint (ha-tool/HTTP/shell actions)
├── heartbeat/
│   ├── HEARTBEAT.md           # Remote heartbeat template (ha-tool)
│   └── routines.md            # Remote cron routine prompts (ha-tool)
│
└── wit/tool.wit               # WASM component interface definition
```

### API Used

Both extensions communicate with Home Assistant via the [HA REST API](https://developers.home-assistant.io/docs/api/rest/). No external services, no cloud relay — all calls go directly to your HA instance.

---

## License

MIT OR Apache-2.0
