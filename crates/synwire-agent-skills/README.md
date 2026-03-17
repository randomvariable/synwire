# synwire-agent-skills

Agent skills runtime for Synwire, implementing the [agentskills.io](https://agentskills.io) specification for discoverable, composable agent capabilities.

## Overview

A skill is a self-contained directory that an agent can discover and invoke. Skills provide natural-language instructions, optional runtime scripts, reference material, and static assets. Synwire extends the base spec with an optional `runtime` field to support scripted execution.

### Directory layout

```text
my-skill/
├── SKILL.md          — manifest (YAML frontmatter) + instruction body
├── scripts/          — optional runtime scripts (Lua, Rhai, WASM)
├── references/       — optional reference material (docs, specs)
└── assets/           — optional static assets (templates, configs)
```

## SKILL.md format

`SKILL.md` begins with a YAML frontmatter block enclosed by `---` delimiters, followed by free-form Markdown instructions.

### Frontmatter fields

| Field | Required | Constraints | Description |
|-------|----------|-------------|-------------|
| `name` | Yes | 1–64 chars, `[a-z0-9-]` | Unique skill identifier |
| `description` | Yes | 1–1024 chars | Human-readable purpose |
| `license` | No | SPDX identifier | e.g. `MIT`, `Apache-2.0` |
| `compatibility` | No | Semver expression | e.g. `>=0.1.0` |
| `allowed-tools` | No | Space-separated tool names | Tools this skill may invoke |
| `runtime` | No | See runtimes table | Synwire extension — how to execute |
| `metadata` | No | `key: value` map | Arbitrary metadata (author, version, tags) |

### Example

```markdown
---
name: summarise-pr
description: "Fetch a GitHub PR, summarise the diff, and post a review comment."
license: MIT
compatibility: ">=0.1.0"
allowed-tools: read write grep semantic_search
runtime: lua
metadata:
  author: example
  version: "1.0.0"
---

## Instructions

Use this skill when asked to review a pull request. Steps:

1. Read the PR diff using the `read` tool.
2. Search for related tests with `semantic_search`.
3. Summarise changes and post a comment.
```

### Name validation

- Lowercase letters (`a-z`), digits (`0-9`), and hyphens (`-`) only
- 1 to 64 characters
- Must match the skill directory name

## Runtimes

| Runtime | `runtime` field value | Description |
|---------|----------------------|-------------|
| Lua | `lua` | Lua scripts via `mlua` (sandboxed) |
| Rhai | `rhai` | Rhai scripts (safe by default, no I/O) |
| WebAssembly | `wasm` | WASM modules via `extism` |
| Tool sequence | `tool-sequence` | Declarative JSON list of tool invocations |
| External | `external` | Script executed as a subprocess |
| None | (omit) | Instruction-only skill (no script execution) |

Omitting `runtime` produces an instruction-only skill: the agent reads the `SKILL.md` body and follows it without any programmatic execution.

## Discovery

Skills are discovered from two locations, in priority order:

| Location | Path | Scope |
|----------|------|-------|
| Project-local | `.<product>/skills/` relative to project root | Project |
| Global | `$DATA/<product>/skills/` | All projects |

Project-local skills take precedence over global skills with the same name. Skills are loaded at agent startup; the registry provides names and descriptions immediately (**progressive disclosure**). Full skill bodies are loaded only when a skill is activated.

```rust,no_run
use synwire_agent_skills::{loader::SkillLoader, registry::SkillRegistry};
use std::path::Path;

let loader = SkillLoader::new();

// Scan global skills directory
let global = loader.scan(Path::new("/home/user/.local/share/myapp/skills")).await?;
// Scan project-local directory
let local  = loader.scan(Path::new(".myapp/skills")).await?;

let mut registry = SkillRegistry::new();
for entry in local.into_iter().chain(global) {
    registry.register(entry);
}

// Progressive disclosure: only names + descriptions loaded
for (name, desc) in registry.list_names_and_descriptions() {
    println!("{name}: {desc}");
}

// Full body loaded on activation
let skill = registry.get("summarise-pr")?;
```

## Progressive disclosure

Loading all skill bodies at startup would consume significant tokens. Instead, the registry loads:

- **At startup**: `name` and `description` from frontmatter only
- **On activation**: full `SKILL.md` body + scripts

This keeps agent context small — typically 50–200 tokens per skill at startup regardless of body length.

## Writing a Lua skill

```lua
-- scripts/summarise_pr.lua
-- Context is provided as a global `ctx` table.

local diff = ctx.tool("fs.read", { path = ctx.args.pr_diff_path })
local results = ctx.tool("code.search_semantic", { query = "related tests" })

return "Summary: " .. diff:sub(1, 500) .. "\n\nRelated: " .. tostring(#results) .. " results"
```

## Writing a Rhai skill

```rhai
// scripts/check_style.rhai
let content = tool("fs.read", #{ path: args.file });
if content.contains("unwrap()") {
    return "Found unwrap() calls — consider handling errors explicitly.";
}
"No issues found."
```

## Writing a tool-sequence skill

A JSON array of tool invocations executed in order. Outputs from earlier steps are available as `$step_N`:

```json
[
  { "tool": "fs.glob",              "args": { "pattern": "**/*.rs" } },
  { "tool": "code.search_semantic","args": { "query": "error handling", "top_k": 5 } }
]
```

## Installing skills

### Global install

Copy the skill directory to `$DATA/<product>/skills/`:

```sh
cp -r my-skill ~/.local/share/myapp/skills/
```

### Project-local install

Copy to `.<product>/skills/` in your project root:

```sh
cp -r my-skill .myapp/skills/
```

### From `synwire-mcp-server`

The MCP server auto-discovers skills from both locations on startup. No restart required after adding a skill — restart the MCP server to pick up new skills.

## Feature flags

| Feature | Description |
|---------|-------------|
| `lua` | Enable Lua runtime via `mlua` |
| `rhai` | Enable Rhai runtime |
| `wasm` | Enable WebAssembly runtime via `extism` |

All runtimes are optional to keep binary size small. Enable only the runtimes your skills require.

```toml
[dependencies]
synwire-agent-skills = { version = "0.1", features = ["lua", "rhai"] }
```
