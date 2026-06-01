# Technical Specification — Token-Optimized Home Assistant MCP Integration

This document outlines the technical design for adapting the Home Assistant integration to the new token-constrained, local-first architecture of IronClaw, as defined in `./.zenflow/tasks/we-changed-a-lot-of-things-in-ou-4056/requirements.md`.

---

## 1. Technical Context

### Language and Platform Decision: Rust vs. C++

The task description requested a comparison between a C++ binary and other approaches for building the Home Assistant Model Context Protocol (MCP) server. We propose using **Rust** for the following key reasons:

1. **Safety & Zero Overhead**: Like C++, Rust compiles to a native executable with no garbage collector, minimal memory overhead, and lightning-fast startup times.
2. **Codebase Reuse**: We can reuse 90% of the logic from `./tools-src/ha-tool/src/api.rs` (such as URL sanitization, date/time logic, state compaction, and network interfaces), which was already written in Rust.
3. **Ecosystem & JSON/HTTP Parsing**: Home Assistant uses complex JSON structures. Writing reliable, memory-safe JSON parsing (`serde_json`) and HTTP request handling in C++ is highly verbose and error-prone, requiring complex external libraries. In Rust, `serde` is robust, safe, and standard-compliant.
4. **Portability and Ease of Compilation**: Compiling Rust is as simple as `cargo build --release`, and it supports Mac, Linux, and Windows with identical code. C++ compilation often breaks across platforms due to compiler and system-level header differences.
5. **No Sandbox Network Issues**: By compiling a local Rust-based MCP server running natively on the host machine (connecting via stdio to IronClaw), we bypass WebAssembly network limitations for local private IP addresses (e.g. `http://192.168.x.x`) seamlessly without needing complex SSRF configuration workarounds.

### Key Dependencies (Rust)
- **`tokio`**: Async runtime for running the stdio MCP loop and fetching HTTP requests concurrently.
- **`serde` & `serde_json`**: Robust JSON serialization and deserialization.
- **`reqwest`** (or `ureq`): Lightweight HTTP client to communicate with Home Assistant's REST API.
- **`strsim`**: String similarity library to implement highly optimized fuzzy matching (e.g., Jaro-Winkler or Levenshtein) for natural-language entity search.

---

## 2. Implementation Approach

To meet the strict token budgets (**≤ 256 tokens** for skill instructions, **8,192 total context budget**), we shift the cognitive load and state size from the LLM to the local MCP server.

### 2.1. Dynamic Entity Caching and Local Fuzzy Search
Instead of the LLM receiving a full list of all smart home entities (which leads to "state explosions" and reasoning failures on small models), we implement host-side pre-filtering:
1. The local MCP server fetches and caches the list of all entities and their states from the `/api/states` endpoint at startup and updates it periodically (e.g., every 5 minutes or on-demand).
2. The LLM never sees the full list.
3. When the user asks a question, the LLM calls `ha_search_entities(query)`.
4. The local server performs fuzzy search against entity IDs, friendly names, and area names using the `strsim` library, scoring and ranking them.
5. The server returns **only the top 1-3 matching entities** in an ultra-condensed format, using **less than 100 tokens** of context.

### 2.2. Unified High-Level Control Layer (`ha_control`)
Instead of exposing dozens of specialized service calls and endpoints (e.g., `climate.set_temperature` vs `light.turn_on` with nested payloads), the LLM is given a single, extremely simple tool `ha_control`.
The MCP server intercepts this tool call and intelligently maps it to the correct Home Assistant service based on the entity domain:
- `ha_control("light.living_room", "turn_on")` -> calls service `light.turn_on`.
- `ha_control("light.living_room", "set_value", 50)` -> maps `50` to `brightness_pct: 50` and calls service `light.turn_on`.
- `ha_control("climate.living_room", "set_value", 21)` -> maps `21` to `temperature: 21` and calls service `climate.set_temperature`.
- `ha_control("switch.plug", "toggle")` -> calls service `switch.toggle`.
- `ha_control("automation.daily_routine", "trigger")` -> calls service `automation.trigger`.

This reduces the LLM's parameter schemas down to a few basic variables, making it trivial for a 3B/7B model to use.

### 2.3. Dual-Format Architecture (WASM + Native MCP)
To maximize flexibility:
- We will organize the codebase so that the core Home Assistant client logic is in a shared library.
- This shared library can be compiled *either* into the native Local MCP Server (`./mcp-server/src/main.rs`) or into the sandboxed WebAssembly component (`./tools-src/ha-tool/src/lib.rs`).
- Native MCP is used for local setups (http), while WASM can be used for public HTTPS secure remote environments.

---

## 3. Source Code Structure Changes

To implement this specification, we will create/edit the following files:

```
ironclaw-home-assistant-skill/
├── ./.zenflow/tasks/...           # Task state and specs
├── ./skills/
│   └── ./skills/SKILL.md        # Optimized to ≤ 256 tokens
├── ./local/
│   └── ./local/skills/SKILL.md  # Optimized to ≤ 256 tokens
├── ./mcp-server/                # New native MCP server crate
│   ├── ./mcp-server/Cargo.toml  # Crate configuration
│   └── ./mcp-server/src/
│       └── ./mcp-server/src/main.rs # MCP stdio server, caching, fuzzy search, and control mapper
└── ./tools-src/ha-tool/         # Existing WASM tool (reused/shared core)
```

---

## 4. Data Model / API / Interface Changes

The MCP server will register exactly 3 tools to the model with extremely lightweight schemas.

### 4.1. Tool 1: `ha_search_entities`
Used by the LLM to search for entities by natural-language query.

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "The search term, friendly name, area name, or keyword (e.g. 'kitchen lamp', 'temperature')."
    },
    "domain": {
      "type": "string",
      "description": "Optional domain filter (e.g. 'light', 'climate', 'switch', 'sensor')."
    }
  },
  "required": ["query"]
}
```

**Response Format**:
```json
[
  {
    "entity_id": "light.kitchen_overhead",
    "name": "Kitchen Overhead Light",
    "state": "off",
    "domain": "light"
  }
]
```

### 4.2. Tool 2: `ha_control`
Used by the LLM to perform actions on a specific entity.

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "entity_id": {
      "type": "string",
      "description": "The target Home Assistant entity_id (e.g. 'light.living_room')."
    },
    "action": {
      "type": "string",
      "enum": ["turn_on", "turn_off", "toggle", "set_value"],
      "description": "The high-level action to perform."
    },
    "value": {
      "type": ["string", "number", "boolean"],
      "description": "Optional value for the action (e.g. 50 for brightness percentage, 21.5 for climate temperature)."
    }
  },
  "required": ["entity_id", "action"]
}
```

**Response Format**:
```json
{
  "status": "success",
  "entity_id": "light.living_room",
  "action": "turn_on"
}
```

### 4.3. Tool 3: `ha_get_diagnostics`
Used by the LLM to verify configuration health or retrieve system alerts.

**Parameters**: None.

**Response Format**:
```json
{
  "status": "online",
  "version": "2026.5.2",
  "errors_detected": false,
  "recent_notifications": []
}
```

---

## 5. Verification Approach

To verify the correctness of the newly introduced MCP server and token-optimized schemas:

1. **Fuzzy Search Unit Tests**: Unit tests inside `./mcp-server/src/main.rs` to verify that various search strings ("bed room thermostat", "living room lights") correctly rank the simulated entities.
2. **Schema Compilation Verification**: Use `cargo test` inside `./mcp-server/` to assert that JSON Schemas for the 3 core tools compile successfully and contain no Ollama-incompatible types.
3. **Execution Tests**: Integration test mock server mocking Home Assistant's REST responses to verify that `ha_control` correctly translates and forwards requests to `/api/services/light/turn_on` and `/api/services/climate/set_temperature`.
4. **Lint and Typecheck**: Run `cargo clippy` and `cargo fmt` to ensure adherence to standard Rust patterns and no warnings or errors.

---

## 6. Remote Host Playwright Integration & E2E Verification

To verify the actual functionality on live, non-simulated infrastructure, we implement a full end-to-end browser execution flow via Playwright.

### 6.1. Target Environment & Credentials
- **Remote Host Machine**: `192.168.10.169` (SSH credentials: `root` / `L1l4pause`)
- **Home Assistant Server**: `http://192.168.19.37:8123`
- **Home Assistant Credentials**: Username `chtugha`, Password `321_homeassistant_123`

### 6.2. Installation & Execution Workflow
1. **Remote Code Deployment**: Deploy and compile `./mcp-server` on the remote host machine (`192.168.10.169`).
2. **IronClaw-UI Config**: Configure the Home Assistant URL and Token on the ironclaw-ui of the remote host.
3. **Playwright Automation Execution**: Launch a Playwright browser instance, navigate to the remote host's ironclaw-ui, login, initiate Home Assistant skill interaction, and verify live responses from the Home Assistant host (`http://192.168.19.37:8123`).
4. **Failsafe Verification**: Run real control actions via Playwright interaction and verify that entity states update correctly in Home Assistant without using mock objects. Any discovered issues on the live integration must be debugged and resolved directly on the target host.

