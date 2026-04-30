# Audit & UX Overhaul Plan

## Investigation Summary

Full audit of all Rust source files (lib.rs, api.rs, shell.rs, types.rs), WIT interface, capabilities JSON, install/build scripts, SKILL.md, HEARTBEAT.md, routines.md, and README.md.

**Rust code finding**: No bugs found. The code is well-structured with comprehensive input validation, proper error handling, thorough test coverage (30+ unit tests), and correct JSON/shell output parsing. The SSH gateway shell commands, base64 encoding, exec output parsing, and HA REST API integrations are all correct.

**UX issues identified**:
1. `HA_URL` is a raw placeholder in HEARTBEAT.md and routines.md that users must manually find-and-replace — no variable, no setup wizard
2. `cargo install cargo-component` is a manual pre-step not handled by install.sh
3. Install script doesn't prompt for or persist the HA URL
4. README is 557 lines with excessive examples and redundant content
5. No single-command installation experience

## Affected Files
- `scripts/install.sh` — rewrite with interactive wizard
- `README.md` — streamline and restructure
- `heartbeat/HEARTBEAT.md` — document auto-substitution
- `heartbeat/routines.md` — document auto-substitution
- `skills/SKILL.md` — optimize and debug

### [x] Step: Audit codebase for bugs and logic faults
- Reviewed all Rust source files, WIT interface, JSON capabilities
- No code bugs found; all parsing, validation, and API logic is correct

### [x] Step: Improve install.sh with interactive wizard
- Prompt for HA URL with validation
- Auto-install cargo-component if missing
- Auto-replace HA_URL placeholder in HEARTBEAT.md and routines.md
- Persist HA URL to config for future use

### [x] Step: Clean up README.md
- Reduce from 557 lines to focused, streamlined docs
- Single-command install experience
- Better structure with clear sections

### [x] Step: Update HEARTBEAT.md and routines.md
- Add notes about auto-substitution by install script

### [x] Step: Verify build and tests

### [x] Step: Audit and optimize SKILL.md against IronClaw codebase
- Read full IronClaw skills codebase: parser, registry, types, CLI, bundled skills
- **Critical bug fixed**: Skill was installed to `~/.ironclaw/skills/home-assistant.SKILL.md` — IronClaw's registry only discovers files named exactly `SKILL.md` in flat or subdirectory layouts. Changed to `~/.ironclaw/skills/home-assistant/SKILL.md`
- **Token optimization**: Reduced prompt from ~1900 tokens to ~861 tokens (55% reduction) by compressing verbose action reference into dense format, removing redundant example JSON calls, inlining SshConfig fields instead of full JSON schema, and merging workflow tips into a concise section
- **Added exclude_keywords**: `memory`, `routine`, `schedule`, `cron`, `commit`, `git`, `code review` — prevents false activation on unrelated conversations
- **Fixed keyword overflow**: Had 24 keywords but IronClaw enforces max 20; silently truncated keywords like `media player`, `notify`, `notification`, `entity` would never match. Consolidated to exactly 20 (merged `light`/`lights`, dropped `blind` and `notify` in favor of `notification`)
- **Lowered max_context_tokens**: 3000 → 2500 (prompt is only ~861 tokens, no need for 3000)
- **Version bumped**: 0.2.0 → 0.3.0
- **Install script**: Added overwrite guard for skill (matching heartbeat/routines pattern), migration cleanup for old wrong path, and skill status tracking in summary output

### [x] Step: Address review feedback (round 4)
- **Skill overwrite guard upgraded**: Changed from blanket skip to version comparison — `extract_skill_version()` reads `version:` from YAML frontmatter in both source and destination; copies only when versions differ, allowing upgrades while still protecting against unnecessary overwrites
- **exclude_keywords refined**: Replaced bare `routine` (which could block legitimate HA queries like "set up a heating routine") with `ironclaw routine`; replaced `schedule` with `cron schedule` to be more specific. `cron` alone is kept since it's purely an IronClaw/system concept
- **Version bumped**: 0.3.0 → 0.3.1
- **Discovery path verification**: Confirmed correct — IronClaw's `discover_from_dir` in `registry.rs` checks `fname == "SKILL.md"`, so subdirectory layout `skills/home-assistant/SKILL.md` is the only valid path

### [x] Step: Investigate API-based skill installation
- **Read full IronClaw API docs** from mintlify (Agent API, Tool API, Workspace API)
- **Read `SkillRegistry`** (`crates/ironclaw_skills/src/registry.rs`) — has `install_skill()`, `prepare_install_to_disk()`, `commit_install()`, `remove_skill()` methods
- **Read `SkillInstallTool`** (`src/tools/builtin/skill_tools.rs`) — agent tool that accepts `name`, `slug`, `url`, or `content` params
- **Read `src/cli/skills.rs`** — CLI only has `list`, `search`, `info` subcommands (no `install`)
- **Read `src/tools/registry.rs`** — `register_skill_tools()` registers `skill_list`, `skill_search`, `skill_install`, `skill_remove` as agent tools

**Finding**: There is **no CLI command** (`ironclaw skills install`) and **no HTTP REST API** for skill installation. The `skill_install` is an **agent tool** — a built-in tool callable only by the LLM during a chat session. It requires a running agent with a SkillRegistry + SkillCatalog wired up.

The `skill_registry` field in `AgentDeps` (`Option<Arc<RwLock<SkillRegistry>>>`) is the internal Rust API, not an HTTP endpoint. Skills are loaded from disk via `discover_all()` at agent startup from three directories:
1. Workspace skills (`<workspace>/skills/`) — Trusted
2. User skills (`~/.ironclaw/skills/`) — Trusted
3. Installed skills (`~/.ironclaw/installed_skills/`) — Installed (lower trust)

**Conclusion**: The file-copy approach in `install.sh` IS the correct and only mechanism for pre-agent skill installation. IronClaw's own bundled skills ship the same way. The `skill_install` agent tool handles dynamic runtime installation during conversations (from ClawHub catalog, GitHub URLs, or raw content). No changes needed to the install script.

### [x] Step: Address review feedback (round 5)
- **Preamble prose corruption fixed**: Added `INSTALL_PREAMBLE` sentinel to HTML comment blocks in HEARTBEAT.md and routines.md. The install script now strips these preamble blocks _before_ URL substitution, so installed copies are clean (no nonsensical "replace http://192.168.1.100:8123 with your actual URL" prose). Source templates retain the comments for manual editors.
- **Verified sed escaping**: `&`, `|`, and `\` are all escaped in the replacement string — tested with edge case URLs
- **Verified tri-state tracking**: `not_found`/`skipped`/`configured` states correctly mapped in summary output for all three file types (skill, heartbeat, routines)
- **Verified extract_skill_version**: Properly placed in helper functions section, empty-version edge case handled with clear warning message
- **All bash syntax validated**: `bash -n` passes clean
