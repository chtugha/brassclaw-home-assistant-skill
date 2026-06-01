# Full SDD workflow

## Configuration
- **Artifacts Path**: {@artifacts_path} → `.zenflow/tasks/{task_id}`

---

## Agent Instructions

---

## Workflow Steps

### [x] Step: Requirements
<!-- chat-id: 9928c138-4497-4fbe-8de1-d2078af84baf -->

Create a Product Requirements Document (PRD) based on the feature description.

1. Review existing codebase to understand current architecture and patterns
2. Analyze the feature definition and identify unclear aspects
3. Ask the user for clarifications on aspects that significantly impact scope or user experience
4. Make reasonable decisions for minor details based on context and conventions
5. If user can't clarify, make a decision, state the assumption, and continue

Focus on **what** the feature should do and **why**, not **how** it should be built. Do not include technical implementation details, technology choices, or code-level decisions — those belong in the Technical Specification.

Save the PRD to `{@artifacts_path}/requirements.md`.

### [x] Step: Technical Specification
<!-- chat-id: 92655641-a47d-4ad4-b313-7ad0e5191a9f -->

Create a technical specification based on the PRD in `{@artifacts_path}/requirements.md`.

1. Review existing codebase architecture and identify reusable components
2. Define the implementation approach

Do not include implementation steps, phases, or task breakdowns — those belong in the Planning step.

Save to `{@artifacts_path}/spec.md` with:
- Technical context (language, dependencies)
- Implementation approach referencing existing code patterns
- Source code structure changes
- Data model / API / interface changes
- Verification approach using project lint/test commands

### [x] Step: Planning
<!-- chat-id: 953f031a-6ab8-4976-97c9-9e363ce287d3 -->

Create a detailed implementation plan based on `{@artifacts_path}/spec.md`.

1. Break down the work into concrete tasks
2. Each task should reference relevant contracts and include verification steps
3. Replace the Implementation step below with the planned tasks

Rule of thumb for step size: each step should represent a coherent unit of work (e.g., implement a component, add an API endpoint). Avoid steps that are too granular (single function) or too broad (entire feature).

Important: unit tests must be part of each implementation task, not separate tasks. Each task should implement the code and its tests together, if relevant.

If the feature is trivial and doesn't warrant full specification, update this workflow to remove unnecessary steps and explain the reasoning to the user.

Save to `{@artifacts_path}/plan.md`.

### [x] Step: Clean Up Version Control and Ignore System Metadata Files
<!-- chat-id: 8b981dc1-cba2-46d8-ad1b-3958bc4cde8e -->
Ensure macOS metadata `.DS_Store` files are untracked and ignored correctly.
- **Goal**: Add `**/.DS_Store` to the `./.gitignore` file and untrack any existing `.DS_Store` files using `git rm --cached` to maintain clean version control boundaries.
- **Verification**: Verify that `.DS_Store` files no longer appear in `git status`.

### [x] Step: Refactor and Isolate Shared Codebase Helpers
<!-- chat-id: a1bc1e2b-ff8e-4a08-bc4a-d0e2ec1e953d -->
Isolate target-agnostic Home Assistant helpers from WASM platform bindings to support the dual-format architecture without compiler conflicts.
- **Goal**: Refactor `./tools-src/ha-tool/` to separate pure, target-agnostic logic (e.g. URL normalization, validation, response truncation) from WASI host-imports (`crate::near::agent::host`) and `wit-bindgen` components. Ensure these are safely accessible or duplicated into `./mcp-server` without compilation issues.
- **Verification**: Verify `./tools-src/ha-tool/` still compiles with `cargo build --target wasm32-wasi` and standardizes shared types.

### [x] Step: Setup the `./mcp-server` Crate and Configuration
<!-- chat-id: 5c5391a9-6a89-43c9-8d64-edde78024c48 -->
Create a new native Rust binary crate `./mcp-server` to act as our Home Assistant MCP server.
- **Goal**: Establish the crate structure, configure `./mcp-server/Cargo.toml` with dependencies (`tokio`, `serde`, `serde_json`, `reqwest`, `strsim`), and verify compilation.
- **Verification**: Run `cargo check` and `cargo test` in `./mcp-server/` to ensure a clean build.

### [x] Step: Implement MCP Protocol Handler
<!-- chat-id: 48835250-81d1-45b0-995b-a9af7d6ab414 -->
Implement the Model Context Protocol (MCP) JSON-RPC standard over standard input/output (stdio) inside `./mcp-server/src/main.rs`.
- **Goal**: Expose the 3 required tools: `ha_search_entities`, `ha_control`, and `ha_get_diagnostics`. Process standard input, parse requests, and output standard output JSON-RPC responses.
- **Verification**: Run standard integration tests with mock input to verify correct JSON-RPC framing and compliance.

### [x] Step: Implement Local Entity Cache and Fuzzy Search
<!-- chat-id: d215dea2-38fa-412e-aea7-2c992b2b1b49 -->
Develop the entity retrieval and fuzzy similarity matching engine.
- **Goal**: Retrieve states from the Home Assistant `/api/states` endpoint, cache them in memory, and implement fuzzy matching using `strsim` within `ha_search_entities` to score and rank friendly names and entity IDs.
- **Verification**: Write unit tests comparing search inputs (e.g., "living room light") against mock entity states to ensure high-accuracy ranking.

### [x] Step: Implement Unified Control Mapper
<!-- chat-id: be0c92c1-207c-4fe3-87f1-878c8a643a8d -->
Create the command mapping and execution layer for the `ha_control` tool.
- **Goal**: Map simple actions (`turn_on`, `turn_off`, `toggle`, `set_value`) to domain-specific Home Assistant REST services (e.g. `light.turn_on` with converted brightness percent parameters, `climate.set_temperature`). Retrieve the HA url and access token from environment variables.
- **Verification**: Add tests using mocked HTTP endpoints to verify correct payload conversion and HTTP POST requests.

### [x] Step: Implement Diagnostics and Log Truncation
<!-- chat-id: c7694dde-048d-41d1-923f-8c0eaabfc20f -->
Implement system diagnostics reporting for the `ha_get_diagnostics` tool.
- **Goal**: Connect to HA's `/api/config` and `/api/error_log` endpoints. Compact any results and truncate logs to guarantee output stays under 500 characters.
- **Verification**: Add verification tests with long logs to confirm strict truncation rules are applied.

### [x] Step: Rewrite and Optimize Skill Instructions
<!-- chat-id: db30104e-95d3-4abd-9e09-7e51701afe2a -->
Rewrite the skill instructions in `./skills/SKILL.md` and `./local/skills/SKILL.md`.
- **Goal**: Strip outdated instructions and write extremely compact, optimized guidelines (≤ 256 tokens total) for using the new simplified MCP tools.
- **Verification**: Confirm that the new instructions strictly describe the 3 simplified MCP tools and fit the tight token budget.

### [x] Step: Final Integration Testing and Cleanup
<!-- chat-id: 254898ea-0adb-4c63-b722-ec83256c3e7e -->
Perform end-to-end local testing, lints, and cleanup.
- **Goal**: Connect the MCP server to a simulated or real Home Assistant instance and verify the complete flow. Clean up formatting and warnings.
- **Verification**: Run `cargo clippy`, `cargo fmt`, and standard tests across the entire workspace.

### [x] Step: Remote Playwright Verification and Integration
<!-- chat-id: 254898ea-0adb-4c63-b722-ec83256c3e7e -->
Install, test, debug, and verify the Home Assistant skill via Playwright browser execution using the ironclaw-ui on the remote test machine (`192.168.10.169`, credentials: `root` / `L1l4pause`).
- **Goal**: Connect to the remote testing host `192.168.10.169`, deploy the Home Assistant skill connecting to the real Home Assistant instance under `http://192.168.19.37:8123` (using username `chtugha`, password `321_homeassistant_123`), and automate end-to-end verification via Playwright browser execution. No simulation is allowed; fix any live issues encountered.
- **Verification**: Verify that the skill works flawlessly end-to-end on the real target machine, correctly searching, controlling, and diagnosing the real Home Assistant instance via the ironclaw-ui interface.

