# Product Requirements Document (PRD) — Token-Optimized Home Assistant Extension for IronClaw

## 1. Executive Summary

### Background
[IronClaw](https://github.com/chtugha/ironclaw) has been redesigned as a native Rust assistant optimized for 100% local operation on consumer hardware. Key constraints of the new system include:
- A hard budget of **8,192 total prompt tokens** (including system prompts, history, skills, and tools).
- A hard budget of **≤ 256 tokens** for local service skills.
- The use of smaller local LLMs (e.g., `llama3.2` 3B, `qwen2.5` 7B/14B) which are prone to confusion when presented with large contexts, verbose API schemas, or multi-step execution flows.

The existing Home Assistant integration relies on large skill instructions (`./skills/SKILL.md` of ~2,000 tokens) and exposes raw, verbose REST JSON payloads directly to the LLM context. This results in frequent context exhaustion, high token costs, and reasoning failures for small local models.

### Goal
Adapt the IronClaw Home Assistant integration to the new token-constrained, local-first architecture. We want to deliver a **high-reliability, low-context-overhead smart home control skill** that works flawlessly with small local models (3B to 14B) on home hardware.

---

## 2. Key Objectives & Why They Matter

1. **Drastically Reduce LLM Prompt Overhead**
   - **Why**: Injecting full Home Assistant service schemas and REST endpoints into the prompt wastes valuable context window. 
   - **What**: Shift the complexity of API generation, formatting, and route handling away from the LLM prompt and into a local helper/protocol integration. Reduce the Home Assistant skill budget to fit comfortably under the **≤ 256 tokens** limit.

2. **Avoid "State Explosions" in LLM Context**
   - **Why**: Querying all states in a typical smart home can return hundreds of entities and hundreds of thousands of characters, easily blowing past the 8,192-token total context limit.
   - **What**: Implement fuzzy search and semantic pre-filtering on the host side. The LLM should never receive the full entity state list. Instead, it queries the integration with user-intent keywords, and receives only the 1–3 relevant entities.

3. **Simplify Action Execution**
   - **Why**: Home Assistant has a complex, multi-layered service-call structure (e.g., `climate.set_temperature` with nested payloads vs `light.turn_on` with brightness/color attributes). Local LLMs struggle to output correct, deeply nested JSON structures consistently.
   - **What**: Provide a unified, high-level control interface. The integration layer should perform smart translation, mapping simple parameters (e.g., `entity_id`, `state`, `value`) to the actual complex HA service payloads.

4. **Standards-Based Extensibility via MCP**
   - **Why**: The Model Context Protocol (MCP) is the standard for exposing tools to AI agents. It shifts tool schemas and inputs out of the standard chat context, registering them as native model tools with concise JSON schemas.
   - **What**: Expose the Home Assistant integration as a local MCP server that IronClaw can connect to natively.

---

## 3. Functional Requirements

### 3.1. Core Features

- **Local Connection & Discovery**:
  - Automatically detect the Home Assistant instance on the local network (with fallback to remote secure HTTPS URLs such as Nabu Casa/DuckDNS).
  - Securely retrieve and authenticate using the user's Long-Lived Access Token.

- **Semantic & Fuzzy Entity Search**:
  - Provide a search interface where the LLM can query for entities using natural language (e.g., "living room lamp", "ac").
  - The search engine must score and return only the top matching entities with a highly condensed state representation (name, state, entity ID, and domain), using less than 150 tokens.

- **Unified Control Interface (`ha_control`)**:
  - A single tool that allows turning on/off, toggling, or setting values (temperatures, brightness, percentages) for any supported domain.
  - The integration layer handles mapping this action to the correct HA service call (e.g., `light.turn_on` vs `switch.turn_on`, or `climate.set_temperature`).

- **Compact System Diagnostics & Monitoring**:
  - Allow checking the configuration validity, fetching condensed error logs (showing only the last few relevant lines), and listing persistent notifications.
  - Optimize the output of these tools to ensure they never exceed 500 characters.

### 3.2. Token Budgets & Constraints

- **Skill Instruction Budget**: The `./skills/SKILL.md` file for Home Assistant must be rewritten and optimized to be **≤ 256 tokens** in total.
- **Tool Schema Budget**: The total schemas for all exposed MCP tools must be concise, avoiding verbose descriptions and redundant parameter definitions.
- **Data Response Budget**: Any tool response returned to the LLM must be strictly capped and formatted.
  - Entity states: Compacted to `entity_id`, `state`, and essential attributes (e.g., `brightness` or `temperature`). All irrelevant metadata (e.g., `context`, `last_updated`, `last_changed`) must be stripped.

---

## 4. User Experience (UX) & Workflows

### 4.1. Installation & Onboarding
1. The user clones the repo or runs a single install script.
2. The installer automatically configures the connection to Home Assistant (local IP or HTTPS URL) and stores the token securely in the local environment.
3. The installer configures IronClaw to load the Home Assistant MCP server.
4. No further user intervention is required; the LLM immediately gains the capability to control the smart home.

### 4.2. Conversational Interacting
- **User**: "Turn off the TV and dim the living room overhead lights to 30%."
- **LLM Agent**:
  1. Calls `ha_search_entities(query="TV")` -> Returns `media_player.living_room_tv` (state: on).
  2. Calls `ha_search_entities(query="living room overhead lights")` -> Returns `light.living_room_overhead` (state: on).
  3. Calls `ha_control(entity_id="media_player.living_room_tv", action="turn_off")`.
  4. Calls `ha_control(entity_id="light.living_room_overhead", action="turn_on", parameters={"brightness_pct": 30})`.
  5. Responds: "I've turned off the TV and dimmed the living room overhead lights to 30%."

---

## 5. Security Requirements

- **No Secret Leakage**: Under no circumstances should the Home Assistant Long-Lived Access Token be exposed to the LLM context, printed to standard logs, or sent to external endpoints.
- **WASM Sandbox Compatibility**: If running in a sandboxed profile (`local-sandbox`), the tool must be able to perform network requests only to the configured Home Assistant host.
- **Safe Mode by Default**: Sensitive operations such as restarting Home Assistant or modifying `/config/configuration.yaml` must require explicit confirmation or be completely disabled by default.
