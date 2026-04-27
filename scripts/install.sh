#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOL_SRC="$ROOT_DIR/tools-src/ha-tool"
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

TOTAL_STEPS=5

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
       [[ "$host_part" == *.home ]] ||
       [[ "$host_part" == *.duckdns.org ]] ||
       [[ "$host_part" == *.nabu.casa ]]; then
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
    echo "  and cron routines. It will be saved for future updates."
    echo ""
    echo "  Examples:"
    echo "    http://homeassistant.local:8123"
    echo "    http://192.168.1.100:8123"
    echo "    https://myha.duckdns.org"
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
            warn "URL does not match a recognized private/local address pattern."
            echo "  Recognized: localhost, 127.0.0.1, 192.168.*, 10.*, 172.16-31.*,"
            echo "              *.local, *.internal, *.lan, *.home, *.duckdns.org, *.nabu.casa"
            echo ""
            printf "  Use this URL anyway (e.g. reverse proxy)? [y/N]: "
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

HEARTBEAT_STATUS="not_found"
ROUTINES_STATUS="not_found"

# --- Pre-flight checks ---

if ! command -v cargo &>/dev/null; then
    error "Rust toolchain not found."
    echo ""
    echo "  Install Rust first:"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    echo "    source \"\$HOME/.cargo/env\""
    echo ""
    exit 1
fi

if ! command -v ironclaw &>/dev/null; then
    error "ironclaw CLI not found."
    echo ""
    echo "  Install IronClaw: https://github.com/nearai/ironclaw"
    echo ""
    exit 1
fi

echo ""
echo "  ${BOLD}IronClaw Home Assistant Extension — Installer${NC}"
echo "  ─────────────────────────────────────────────"

# --- Step 1: Ensure build dependencies ---

step 1 "Checking build dependencies..."

if ! cargo component --version &>/dev/null 2>&1; then
    info "Installing cargo-component (required for WASM builds)..."
    cargo install cargo-component
else
    info "cargo-component already installed."
fi

if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then
    info "Adding wasm32-wasip2 target..."
    rustup target add wasm32-wasip2
else
    info "wasm32-wasip2 target already present."
fi

# --- Step 2: Prompt for Home Assistant URL ---

step 2 "Configuring Home Assistant URL..."
prompt_ha_url
info "URL saved: $HA_URL"

# --- Step 3: Install ha-tool ---

step 3 "Installing ha-tool from source..."
ironclaw tool install "$TOOL_SRC"

# --- Step 4: Install optional files ---

step 4 "Installing skill and heartbeat files..."

SKILL_SRC="$ROOT_DIR/skills/SKILL.md"
SKILL_DEST_DIR="$IRONCLAW_DIR/skills/home-assistant"
SKILL_DEST="$SKILL_DEST_DIR/SKILL.md"
SKILL_STATUS="not_found"
OLD_SKILL_PATH="$IRONCLAW_DIR/skills/home-assistant.SKILL.md"
if [[ -f "$OLD_SKILL_PATH" ]]; then
    rm -f "$OLD_SKILL_PATH"
    info "Removed old skill file at wrong path: $OLD_SKILL_PATH"
fi
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
            info "Upgraded skill: $dest_ver → $src_ver"
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
    warn "No SKILL.md found — skipping (tool still works via auto-discovery)."
fi

HEARTBEAT_SRC="$ROOT_DIR/heartbeat/HEARTBEAT.md"
HEARTBEAT_DEST="$IRONCLAW_DIR/HEARTBEAT.md"
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

ROUTINES_SRC="$ROOT_DIR/heartbeat/routines.md"
ROUTINES_DEST="$IRONCLAW_DIR/routines.md"
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

# --- Step 5: Configure HA token ---

step 5 "Configuring Home Assistant access token..."
echo ""
echo "  Create a long-lived access token in Home Assistant:"
echo "    1. Open ${BOLD}${HA_URL}/profile${NC} in your browser"
echo "    2. Scroll to ${BOLD}Long-Lived Access Tokens${NC}"
echo "    3. Click ${BOLD}Create Token${NC}, name it (e.g. 'ironclaw'), copy the token"
echo ""
ironclaw tool auth ha-tool

# --- Done ---

echo ""
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "  ${GREEN}  ✓ ha-tool installed successfully!${NC}"
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  ${BOLD}Verify:${NC}"
echo "    ironclaw tool list"
echo "    ironclaw tool info ha-tool"
echo ""
echo "  ${BOLD}Quick test:${NC}"
echo "    ironclaw chat"
echo "    > Is my Home Assistant at ${HA_URL} online?"
echo ""
echo "  ${BOLD}Configuration saved:${NC}"
echo "    HA URL:     $HA_URL_FILE"
case "$SKILL_STATUS" in
    configured) echo "    Skill:      $SKILL_DEST" ;;
    skipped)    echo "    Skill:      $SKILL_DEST (skipped — already exists)" ;;
    *)          echo "    Skill:      not installed (source template not found)" ;;
esac
case "$HEARTBEAT_STATUS" in
    configured) echo "    Heartbeat:  $HEARTBEAT_DEST" ;;
    skipped)    echo "    Heartbeat:  $HEARTBEAT_DEST (skipped — already exists)" ;;
    *)          echo "    Heartbeat:  not installed (source template not found)" ;;
esac
case "$ROUTINES_STATUS" in
    configured) echo "    Routines:   $ROUTINES_DEST" ;;
    skipped)    echo "    Routines:   $ROUTINES_DEST (skipped — already exists)" ;;
    *)          echo "    Routines:   not installed (source template not found)" ;;
esac
echo ""
