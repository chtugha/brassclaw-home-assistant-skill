#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$(cd "$LOCAL_DIR/.." && pwd)"
IRONCLAW_DIR="${HOME}/.ironclaw"
HA_URL_FILE="$IRONCLAW_DIR/.ha_url"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

info()  { printf "${GREEN}==>${NC} %s\n" "$1"; }
warn()  { printf "${YELLOW}WARNING:${NC} %s\n" "$1"; }
error() { printf "${RED}ERROR:${NC} %s\n" "$1" >&2; }
step()  { printf "\n${BOLD}[%s/%s]${NC} %s\n" "$1" "$TOTAL_STEPS" "$2"; }

TOTAL_STEPS=3

validate_ha_url() {
    local url="$1"
    local lower
    lower="$(echo "$url" | tr '[:upper:]' '[:lower:]')"

    if [[ ! "$lower" =~ ^https?:// ]]; then
        return 1
    fi

    local host_part="${lower#*://}"
    host_part="${host_part%%/*}"
    host_part="${host_part%%:*}"

    if [[ -z "$host_part" ]]; then
        return 1
    fi

    if [[ "$host_part" == "localhost" ]] ||
       [[ "$host_part" == "127.0.0.1" ]] ||
       [[ "$host_part" =~ ^192\.168\. ]] ||
       [[ "$host_part" =~ ^10\. ]] ||
       [[ "$host_part" =~ ^172\.(1[6-9]|2[0-9]|3[01])\. ]] ||
       [[ "$host_part" == *.local ]] ||
       [[ "$host_part" == *.internal ]] ||
       [[ "$host_part" == *.lan ]] ||
       [[ "$host_part" == *.home ]]; then
        return 0
    fi

    return 1
}

prompt_ha_url() {
    local saved_url=""
    if [[ -f "$HA_URL_FILE" ]]; then
        saved_url="$(cat "$HA_URL_FILE" 2>/dev/null || true)"
    fi

    echo ""
    echo "  Your Home Assistant base URL is needed for heartbeat monitoring"
    echo "  and cron routines."
    echo ""
    echo "  Examples:"
    echo "    http://homeassistant.local:8123"
    echo "    http://192.168.1.100:8123"
    echo "    http://localhost:8123"
    echo ""

    local url=""
    while true; do
        if [[ -n "$saved_url" ]]; then
            printf "  Home Assistant URL [${BOLD}%s${NC}]: " "$saved_url"
        else
            printf "  Home Assistant URL: "
        fi
        read -r url
        if [[ -z "$url" && -n "$saved_url" ]]; then
            url="$saved_url"
        fi
        if [[ -z "$url" ]]; then
            warn "URL cannot be empty."
            continue
        fi
        url="${url%/}"
        if validate_ha_url "$url"; then
            break
        else
            warn "URL does not look like a local HA address."
            echo "  This installer is for local HA instances (http://, private IPs, *.local)."
            echo "  For public HTTPS HA (Nabu Casa, DuckDNS), use the remote installer instead:"
            echo "    ${BOLD}./scripts/install.sh${NC}"
            echo ""
            printf "  Use this URL anyway? [y/N]: "
            read -r override
            if [[ "$override" =~ ^[Yy]$ ]]; then
                break
            fi
        fi
    done

    mkdir -p "$IRONCLAW_DIR"
    echo "$url" > "$HA_URL_FILE"
    HA_URL="$url"
}

extract_skill_version() {
    local file="$1"
    grep -m1 '^version:' "$file" 2>/dev/null | sed 's/^version:[[:space:]]*//' | tr -d '[:space:]'
}

replace_ha_url_placeholder() {
    local file="$1"
    local url="$2"
    if [[ -f "$file" ]]; then
        if grep -q '{{HA_URL}}' "$file" 2>/dev/null; then
            local escaped_url
            escaped_url="$(printf '%s' "$url" | sed 's/[&|\\]/\\&/g')"
            if [[ "$(uname)" == "Darwin" ]]; then
                sed -i '' '/<!-- INSTALL_PREAMBLE:/,/-->/d' "$file"
                sed -i '' "s|{{HA_URL}}|${escaped_url}|g" "$file"
            else
                sed -i '/<!-- INSTALL_PREAMBLE:/,/-->/d' "$file"
                sed -i "s|{{HA_URL}}|${escaped_url}|g" "$file"
            fi
        fi
    fi
}

if ! command -v ironclaw &>/dev/null; then
    error "ironclaw CLI not found."
    echo ""
    echo "  Install IronClaw: https://github.com/nearai/ironclaw"
    echo ""
    exit 1
fi

SHELL_TOOL_AVAILABLE=false
if ironclaw tool list 2>/dev/null | grep -q '\bshell\b'; then
    SHELL_TOOL_AVAILABLE=true
fi

echo ""
echo "  ${BOLD}IronClaw Home Assistant Extension — Local Installer${NC}"
echo "  ────────────────────────────────────────────────────"
echo ""
echo "  This installer sets up the local HA extension (shell+curl)."
echo "  No WASM tool or build step required."

if [[ "$SHELL_TOOL_AVAILABLE" != "true" ]]; then
    echo ""
    echo "  ${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "  ${YELLOW}  WARNING: Built-in 'shell' tool not found${NC}"
    echo "  ${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "  The local extension requires IronClaw's built-in 'shell' tool,"
    echo "  which is only available when ${BOLD}allow_local_tools = true${NC}."
    echo ""
    echo "  This is the default for ${BOLD}ironclaw chat${NC} (CLI mode), but"
    echo "  server/relay deployments disable it by default."
    echo ""
    echo "  To fix:"
    echo "    • CLI mode: the shell tool should be available automatically."
    echo "      Run ${BOLD}ironclaw tool list${NC} to verify."
    echo "    • Server mode: set ${BOLD}ALLOW_LOCAL_TOOLS=true${NC} in your"
    echo "      environment or IronClaw config, then restart the server."
    echo ""
    echo "  Alternatively, expose HA via HTTPS (Nabu Casa or DuckDNS)"
    echo "  and use the remote installer instead: ${BOLD}./scripts/install.sh${NC}"
    echo ""
    printf "  Continue anyway? [y/N]: "
    read -r cont
    if [[ ! "$cont" =~ ^[Yy]$ ]]; then
        echo ""
        info "Installation cancelled."
        exit 0
    fi
fi

# --- Step 1: Prompt for Home Assistant URL ---

step 1 "Configuring Home Assistant URL..."
prompt_ha_url
info "URL saved: $HA_URL"

# --- Step 2: Install skill, heartbeat, and routines ---

step 2 "Installing skill and heartbeat files..."

REMOTE_SKILL_DIR="$IRONCLAW_DIR/skills/home-assistant"
if [[ -d "$REMOTE_SKILL_DIR" ]]; then
    warn "Remote extension skill found at $REMOTE_SKILL_DIR"
    echo "  The local and remote extensions have overlapping keywords."
    echo "  Running both wastes token budget. Removing remote skill..."
    rm -rf "$REMOTE_SKILL_DIR"
    info "Removed remote skill: $REMOTE_SKILL_DIR"
fi

SKILL_SRC="$LOCAL_DIR/skills/SKILL.md"
SKILL_DEST_DIR="$IRONCLAW_DIR/skills/home-assistant-local"
SKILL_DEST="$SKILL_DEST_DIR/SKILL.md"
SKILL_STATUS="not_found"
if [[ -f "$SKILL_SRC" ]]; then
    if [[ -f "$SKILL_DEST" ]]; then
        src_ver="$(extract_skill_version "$SKILL_SRC")"
        dest_ver="$(extract_skill_version "$SKILL_DEST")"
        if [[ -z "$src_ver" ]]; then
            warn "Could not read version from source SKILL.md — skipping update."
            SKILL_STATUS="skipped"
        elif [[ "$src_ver" != "$dest_ver" ]]; then
            mkdir -p "$SKILL_DEST_DIR"
            cp "$SKILL_SRC" "$SKILL_DEST"
            SKILL_STATUS="configured"
            info "Upgraded skill: $dest_ver -> $src_ver"
        else
            SKILL_STATUS="skipped"
            info "SKILL.md already at version $dest_ver — no update needed."
        fi
    else
        mkdir -p "$SKILL_DEST_DIR"
        cp "$SKILL_SRC" "$SKILL_DEST"
        SKILL_STATUS="configured"
        info "Installed skill: $SKILL_DEST"
    fi
else
    warn "No SKILL.md found in local/skills/ — skipping."
fi

HEARTBEAT_SRC="$LOCAL_DIR/heartbeat/HEARTBEAT.md"
HEARTBEAT_DEST="$IRONCLAW_DIR/HEARTBEAT.md"
HEARTBEAT_STATUS="not_found"
if [[ -f "$HEARTBEAT_SRC" ]]; then
    if [[ -f "$HEARTBEAT_DEST" ]]; then
        HEARTBEAT_STATUS="skipped"
        warn "HEARTBEAT.md already exists — leaving unchanged."
        echo "    (Merge entries from $HEARTBEAT_SRC manually if desired.)"
    else
        cp "$HEARTBEAT_SRC" "$HEARTBEAT_DEST"
        replace_ha_url_placeholder "$HEARTBEAT_DEST" "$HA_URL"
        HEARTBEAT_STATUS="configured"
        info "Installed and configured: $HEARTBEAT_DEST"
    fi
else
    warn "No HEARTBEAT.md found — skipping."
fi

ROUTINES_SRC="$LOCAL_DIR/heartbeat/routines.md"
ROUTINES_DEST="$IRONCLAW_DIR/routines.md"
ROUTINES_STATUS="not_found"
if [[ -f "$ROUTINES_SRC" ]]; then
    if [[ -f "$ROUTINES_DEST" ]]; then
        ROUTINES_STATUS="skipped"
        warn "routines.md already exists — leaving unchanged."
        echo "    (Merge entries from $ROUTINES_SRC manually if desired.)"
    else
        cp "$ROUTINES_SRC" "$ROUTINES_DEST"
        replace_ha_url_placeholder "$ROUTINES_DEST" "$HA_URL"
        ROUTINES_STATUS="configured"
        info "Installed and configured: $ROUTINES_DEST"
    fi
fi

# --- Step 3: Store HA token ---

step 3 "Configuring Home Assistant access token..."
echo ""
echo "  The agent needs a long-lived access token to call the HA REST API."
echo "  Create one in Home Assistant:"
echo "    1. Open ${BOLD}${HA_URL}/profile${NC} in your browser"
echo "    2. Scroll to ${BOLD}Long-Lived Access Tokens${NC}"
echo "    3. Click ${BOLD}Create Token${NC}, name it (e.g. 'ironclaw'), copy the token"
echo ""
echo "  The token will be stored in IronClaw's encrypted secret store."
echo ""

TOKEN_FILE="$IRONCLAW_DIR/.ha_token"
saved_token=""
if [[ -f "$TOKEN_FILE" ]]; then
    saved_token="$(cat "$TOKEN_FILE" 2>/dev/null || true)"
fi

while true; do
    if [[ -n "$saved_token" ]]; then
        printf "  HA Token [${BOLD}*****${NC} — press Enter to keep]: "
    else
        printf "  HA Token: "
    fi
    read -r -s token
    echo ""
    if [[ -z "$token" && -n "$saved_token" ]]; then
        token="$saved_token"
    fi
    if [[ -z "$token" ]]; then
        warn "Token cannot be empty."
        continue
    fi
    break
done

mkdir -p "$IRONCLAW_DIR"
echo "$token" > "$TOKEN_FILE"
chmod 600 "$TOKEN_FILE"
info "Token saved: $TOKEN_FILE"

# --- Done ---

echo ""
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "  ${GREEN}  ✓ Local HA extension installed!${NC}"
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  ${BOLD}Quick test:${NC}"
echo "    ironclaw chat"
echo "    > Is my Home Assistant at ${HA_URL} online?"
echo ""
echo "  The agent will use shell+curl to call your HA REST API directly."
echo "  No WASM sandbox restrictions — works with any local HA instance."
echo ""
echo "  ${BOLD}Requirement:${NC} IronClaw's built-in 'shell' tool must be available."
echo "  In CLI mode (ironclaw chat) it is enabled by default."
echo "  In server mode, set ${BOLD}ALLOW_LOCAL_TOOLS=true${NC} in your config."
if [[ "$SHELL_TOOL_AVAILABLE" != "true" ]]; then
    echo ""
    echo "  ${YELLOW}⚠  The 'shell' tool was NOT detected during install.${NC}"
    echo "  ${YELLOW}   See the warning above for how to enable it.${NC}"
fi
echo ""
echo "  ${BOLD}Configuration saved:${NC}"
echo "    HA URL:     $HA_URL_FILE"
echo "    HA Token:   $TOKEN_FILE"
case "$SKILL_STATUS" in
    configured) echo "    Skill:      $SKILL_DEST" ;;
    skipped)    echo "    Skill:      $SKILL_DEST (skipped — already up to date)" ;;
    *)          echo "    Skill:      not installed (source not found)" ;;
esac
case "$HEARTBEAT_STATUS" in
    configured) echo "    Heartbeat:  $HEARTBEAT_DEST" ;;
    skipped)    echo "    Heartbeat:  $HEARTBEAT_DEST (skipped — already exists)" ;;
    *)          echo "    Heartbeat:  not installed (source not found)" ;;
esac
case "$ROUTINES_STATUS" in
    configured) echo "    Routines:   $ROUTINES_DEST" ;;
    skipped)    echo "    Routines:   $ROUTINES_DEST (skipped — already exists)" ;;
    *)          echo "    Routines:   not installed (source not found)" ;;
esac
echo ""
