# Auto

## Configuration
- **Artifacts Path**: {@artifacts_path} → `.zenflow/tasks/{task_id}`

## Agent Instructions

Ask the user questions when anything is unclear or needs their input. This includes:
- Ambiguous or incomplete requirements
- Technical decisions that affect architecture or user experience
- Trade-offs that require business context

Do not make assumptions on important decisions — get clarification first.

**Debug requests, questions, and investigations:** answer or investigate first. Do not create a plan upfront — the user needs an answer, not a plan. A plan may become relevant later once the investigation reveals what needs to change.

**For all other tasks**, before writing any code, assess the scope of the actual change (not the prompt length — a one-sentence prompt can describe a large feature). Scale your approach:

- **Trivial** (typo, config tweak, single obvious change): implement directly, no plan needed.
- **Small** (a few files, clear what to do): write 2–3 sentences in `./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/plan.md` describing what and why, then implement. No substeps.
- **Medium** (multiple components, design decisions, edge cases): write a plan in `./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/plan.md` with requirements, affected files, key decisions, verification. Break into 3–5 steps.
- **Large** (new feature, cross-cutting, unclear scope): gather requirements and write a technical spec first (`./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/requirements.md`, `./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/spec.md`). Then write `./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/plan.md` with concrete steps referencing the spec.

**Skip planning and implement directly when** the task is trivial, or the user explicitly asks to "just do it" / gives a clear direct instruction.

To reflect the actual purpose of the first step, you can rename it to something more relevant (e.g., Planning, Investigation). Do NOT remove meta information like comments for any step.

Rule of thumb for step size: each step = a coherent unit of work (component, endpoint, test suite). Not too granular (single function), not too broad (entire feature). Unit tests are part of each step, not separate.

Update `./.zenflow/tasks/our-ironclaw-fork-was-rebranded-6a40/plan.md` if it makes sense to have a plan and task has more than 1 big step.

## Implementation Steps

### [x] Step 1: Sync local codebase with remote GitHub repo
- Check if local is newer than remote
- Push or pull accordingly

### [x] Step 2: Rebrand files from ironclaw to brassclaw
- Change folder names and file contents in local repo

### [x] Step 3: Update local documentation and installation scripts
- Update all occurrences of ironclaw to brassclaw

### [x] Step 4: Commit and push rebranded changes to GitHub main
- Execute git commit and push

### [x] Step 5: Rename the GitHub repository to brassclaw-home-assistant-skill
- Use GitHub API or CLI to rename repo

### [x] Step 6: Update documentation and scripts to the new path and push to GitHub
- Ensure path references are updated and pushed

### [x] Step 7: Analyze brassclaw repository skill installation mechanism and implement support in our codebase
- Scan https://github.com/chtugha/brassclaw
- Add necessary code to support name & url skill installation

### [ ] Step 8: Commit and push installation feature changes to GitHub
- Execute git commit and push

### [ ] Step 9: Test installation and functionality on real testing machine
- Test on 192.168.10.169
- Test Home Assistant integration on 192.168.19.37
