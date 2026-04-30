#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
IRONCLAW_DIR="${HOME}/.ironclaw"
HA_URL_FILE="$IRONCLAW_DIR/.ha_url"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

info()  { printf "${GREEN}==>${NC} %s\n" "$1"; }
warn()  { printf "${YELLOW}WARNING:${NC} %s\n" "$1"; }
error() { printf "${RED}ERROR:${NC} %s\n" "$1" >&2; }
step()  { printf "\n${BOLD}[%s/%s]${NC} %s\n" "$1" "$TOTAL_STEPS" "$2"; }

TOTAL_STEPS=5

supervisor_api() {
    local method="$1" path="$2" ha_url="$3" token="$4"
    shift 4
    local url="${ha_url}/api/hassio${path}"
    local args=(-s -S -o /dev/fd/3 -w "%{http_code}" \
        -H "Authorization: Bearer ${token}" \
        -H "Content-Type: application/json" \
        -X "$method")
    if [[ $# -gt 0 ]]; then
        args+=(-d "$1")
    fi
    local body="" http_code=""
    exec 3>&1
    http_code=$(curl "${args[@]}" "$url") || { exec 3>&-; return 1; }
    exec 3>&-
    echo "$http_code"
}

supervisor_api_full() {
    local method="$1" path="$2" ha_url="$3" token="$4"
    shift 4
    local url="${ha_url}/api/hassio${path}"
    local args=(-s -S \
        -H "Authorization: Bearer ${token}" \
        -H "Content-Type: application/json" \
        -X "$method")
    if [[ $# -gt 0 ]]; then
        args+=(-d "$1")
    fi
    curl "${args[@]}" "$url" 2>/dev/null
}

echo ""
echo "  ${BOLD}DuckDNS + Let's Encrypt — HTTPS Setup for Home Assistant${NC}"
echo "  ──────────────────────────────────────────────────────────"
echo ""
echo "  This script configures the DuckDNS add-on on your HA OS instance"
echo "  with automatic Let's Encrypt TLS certificates. After setup, your"
echo "  HA instance will be reachable at https://<domain>.duckdns.org with"
echo "  a valid certificate — compatible with the IronClaw sandbox."
echo ""
echo "  ${BOLD}Prerequisites:${NC}"
echo "    - Home Assistant OS (Supervisor required)"
echo "    - A free DuckDNS account (https://www.duckdns.org)"
echo "    - A long-lived access token from HA"
echo "    - Port 443 forwarded to your HA host (for initial cert issuance)"
echo ""

# --- Step 1: Collect HA connection details ---

step 1 "Home Assistant connection details"

HA_URL=""
if [[ -f "$HA_URL_FILE" ]]; then
    HA_URL="$(cat "$HA_URL_FILE" 2>/dev/null || true)"
fi

if [[ -n "$HA_URL" ]]; then
    printf "  HA URL [${BOLD}%s${NC}]: " "$HA_URL"
    read -r input_url
    if [[ -n "$input_url" ]]; then
        HA_URL="${input_url%/}"
    fi
else
    printf "  HA URL (e.g. http://192.168.1.100:8123): "
    read -r HA_URL
    HA_URL="${HA_URL%/}"
fi

if [[ -z "$HA_URL" ]]; then
    error "HA URL is required."
    exit 1
fi

echo ""
echo "  Enter a long-lived access token from HA."
echo "  (Create one at ${HA_URL}/profile → Long-Lived Access Tokens)"
echo ""
printf "  HA Token: "
read -rs HA_TOKEN
echo ""

if [[ -z "$HA_TOKEN" ]]; then
    error "HA token is required."
    exit 1
fi

info "Testing connection to ${HA_URL}..."
http_code=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer ${HA_TOKEN}" \
    "${HA_URL}/api/" 2>/dev/null) || true

if [[ "$http_code" != "200" ]]; then
    error "Cannot reach HA API (HTTP ${http_code:-timeout}). Check URL and token."
    exit 1
fi
info "HA API reachable."

# --- Step 2: Check Supervisor availability ---

step 2 "Checking for HA Supervisor..."

supervisor_check=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer ${HA_TOKEN}" \
    "${HA_URL}/api/hassio/info" 2>/dev/null) || true

if [[ "$supervisor_check" != "200" ]]; then
    warn "Supervisor API not available (HTTP ${supervisor_check:-timeout})."
    echo ""
    echo "  This script requires Home Assistant OS with the Supervisor."
    echo "  For HA Container or HA Core, use certbot manually:"
    echo ""
    echo "  ${BOLD}1. Install certbot:${NC}"
    echo "     pip install certbot certbot-dns-duckdns"
    echo ""
    echo "  ${BOLD}2. Obtain certificate:${NC}"
    echo "     certbot certonly --authenticator duckdns \\"
    echo "       --duckdns-token=YOUR_DUCKDNS_TOKEN \\"
    echo "       -d yourdomain.duckdns.org"
    echo ""
    echo "  ${BOLD}3. Add to configuration.yaml:${NC}"
    echo "     http:"
    echo "       ssl_certificate: /path/to/fullchain.pem"
    echo "       ssl_key: /path/to/privkey.pem"
    echo ""
    echo "  ${BOLD}4. Restart Home Assistant${NC}"
    echo ""
    exit 0
fi
info "Supervisor available."

# --- Step 3: Collect DuckDNS details ---

step 3 "DuckDNS configuration"

echo ""
echo "  Create a free subdomain at ${BOLD}https://www.duckdns.org${NC}"
echo "  Your token is shown at the top of the DuckDNS dashboard."
echo ""

printf "  DuckDNS subdomain (without .duckdns.org): "
read -r DUCK_SUBDOMAIN

if [[ -z "$DUCK_SUBDOMAIN" ]]; then
    error "Subdomain is required."
    exit 1
fi

DUCK_SUBDOMAIN="${DUCK_SUBDOMAIN%.duckdns.org}"
DUCK_DOMAIN="${DUCK_SUBDOMAIN}.duckdns.org"

printf "  DuckDNS token: "
read -rs DUCK_TOKEN
echo ""

if [[ -z "$DUCK_TOKEN" ]]; then
    error "DuckDNS token is required."
    exit 1
fi

info "Domain: ${DUCK_DOMAIN}"

# --- Step 4: Install and configure DuckDNS add-on ---

step 4 "Installing DuckDNS add-on..."

addon_info=$(supervisor_api_full GET "/addons/core_duckdns/info" "$HA_URL" "$HA_TOKEN")
addon_installed=$(echo "$addon_info" | grep -o '"version":"[^"]*"' | head -1 || true)

if [[ -z "$addon_installed" || "$addon_info" == *'"version":null'* ]]; then
    info "Installing DuckDNS add-on from store..."
    install_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer ${HA_TOKEN}" \
        -H "Content-Type: application/json" \
        -X POST "${HA_URL}/api/hassio/store/addons/core_duckdns/install" 2>/dev/null) || true

    if [[ "$install_code" != "200" ]]; then
        error "Failed to install DuckDNS add-on (HTTP ${install_code})."
        echo "  Try installing it manually: Settings → Add-ons → DuckDNS"
        exit 1
    fi
    info "DuckDNS add-on installed."
else
    info "DuckDNS add-on already installed."
fi

info "Configuring DuckDNS add-on..."
options_payload=$(cat <<EOF
{
  "options": {
    "domains": ["${DUCK_DOMAIN}"],
    "token": "${DUCK_TOKEN}",
    "aliases": [],
    "lets_encrypt": {
      "accept_terms": true,
      "algo": "secp384r1",
      "certfile": "fullchain.pem",
      "keyfile": "privkey.pem"
    },
    "seconds": 300
  }
}
EOF
)

config_code=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer ${HA_TOKEN}" \
    -H "Content-Type: application/json" \
    -X POST -d "$options_payload" \
    "${HA_URL}/api/hassio/addons/core_duckdns/options" 2>/dev/null) || true

if [[ "$config_code" != "200" ]]; then
    error "Failed to configure DuckDNS add-on (HTTP ${config_code})."
    exit 1
fi
info "DuckDNS add-on configured with Let's Encrypt enabled."

info "Starting DuckDNS add-on..."
start_code=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer ${HA_TOKEN}" \
    -H "Content-Type: application/json" \
    -X POST "${HA_URL}/api/hassio/addons/core_duckdns/start" 2>/dev/null) || true

if [[ "$start_code" != "200" ]]; then
    error "Failed to start DuckDNS add-on (HTTP ${start_code})."
    echo "  Try starting it manually: Settings → Add-ons → DuckDNS → Start"
    exit 1
fi
info "DuckDNS add-on started."

# --- Step 5: Final configuration steps ---

step 5 "Remaining manual steps"

echo ""
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "  ${GREEN}  DuckDNS add-on installed, configured, and running!${NC}"
echo "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  ${BOLD}Complete these steps to enable HTTPS:${NC}"
echo ""
echo "  ${BOLD}1. Add to your configuration.yaml:${NC}"
echo ""
echo "     ${DIM}http:${NC}"
echo "     ${DIM}  ssl_certificate: /ssl/fullchain.pem${NC}"
echo "     ${DIM}  ssl_key: /ssl/privkey.pem${NC}"
echo ""
echo "  ${BOLD}2. Forward port 443 on your router to your HA host${NC}"
echo "     (required for Let's Encrypt certificate issuance)"
echo ""
echo "  ${BOLD}3. Restart Home Assistant${NC}"
echo "     Settings → System → Restart"
echo ""
echo "  ${BOLD}4. Access HA via:${NC}"
echo "     ${GREEN}https://${DUCK_DOMAIN}${NC}"
echo ""
echo "  ${BOLD}5. Update your IronClaw HA URL:${NC}"
echo "     Re-run ${BOLD}scripts/install.sh${NC} and enter"
echo "     ${GREEN}https://${DUCK_DOMAIN}${NC} as the URL."
echo ""
echo "  ${YELLOW}Note:${NC} The first certificate may take a few minutes to issue."
echo "  Check the DuckDNS add-on logs if HTTPS doesn't work immediately."
echo ""

DUCK_URL="https://${DUCK_DOMAIN}"
mkdir -p "$IRONCLAW_DIR"
echo "$DUCK_URL" > "$HA_URL_FILE"
info "Saved new HA URL: ${DUCK_URL}"
echo ""
