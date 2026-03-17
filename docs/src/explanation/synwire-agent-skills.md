# synwire-agent-skills: Composable Agent Skills

`synwire-agent-skills` implements the [agentskills.io](https://agentskills.io) specification for discoverable, composable agent skills, extended with Synwire-specific runtime hints. A skill is a self-contained unit of agent capability --- instructions plus optional executable code --- that can be loaded, registered, and invoked at runtime.

## Why a separate crate?

Skills occupy a unique position between tools and prompts. A tool is a function with a JSON Schema; a prompt is static text. A skill is both: it carries human-readable instructions (injected into the LLM context) and may also contain executable scripts. Separating skills into their own crate keeps the core tool/agent abstractions clean while allowing the skills runtime to evolve independently.

## Skill directory structure

Each skill is a directory containing:

```text
my-skill/
  SKILL.md        # Required: YAML frontmatter (manifest) + Markdown body (instructions)
  scripts/        # Optional: runtime scripts (Lua, Rhai, WASM, shell)
  references/     # Optional: reference material the LLM can consult
  assets/         # Optional: static assets
```

Skills are discovered from two locations:

- **Global:** `$DATA/<product>/skills/` --- shared across all projects
- **Project-local:** `.<product>/skills/` --- project-specific skills

## Key types

### `SkillManifest`

Parsed from the YAML frontmatter of `SKILL.md`. Fields include:

| Field | Type | Description |
|---|---|---|
| `name` | `String` | 1--64 chars, lowercase letters, digits, hyphens |
| `description` | `String` | 1--1024 chars, human-readable summary |
| `license` | `Option<String>` | SPDX identifier |
| `compatibility` | `Option<String>` | Semver expression |
| `metadata` | `HashMap<String, String>` | Arbitrary key-value pairs |
| `allowed_tools` | `Vec<String>` | Tools this skill is permitted to invoke |
| `runtime` | `Option<SkillRuntime>` | Synwire extension: execution runtime hint |

### `SkillRuntime`

A `#[non_exhaustive]` enum specifying how a skill's scripts execute:

- **`Lua`** --- Lua scripting via `mlua` (feature: `lua-runtime`)
- **`Rhai`** --- Rhai scripting (feature: `rhai-runtime`)
- **`Wasm`** --- WebAssembly via `extism` (feature: `wasm-runtime`)
- **`ToolSequence`** --- a declarative sequence of tool invocations (always available)
- **`External`** --- an external process (always available)

### `SkillLoader`

Scans a directory for immediate child directories containing `SKILL.md`, parses each manifest, extracts the body text, and validates structural invariants (e.g. the directory name must match `manifest.name`).

### `SkillRegistry`

An in-memory registry supporting **progressive disclosure**: callers can list skill names and descriptions cheaply (for tool search indexing), then retrieve the full body only when a skill is activated. This pattern mirrors the `ToolSearchIndex` approach in `synwire-core`.

### `SkillExecutor`

The common trait implemented by all runtime variants:

```rust,no_run
pub trait SkillExecutor: Send + Sync {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError>;
    fn execute_with_context(
        &self, input: SkillInput, context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError>;
}
```

When a `SkillContext` is provided, runtimes that support it expose filesystem operations scoped to the project root, tool invocation via `ToolProvider`, and LLM access via `SamplingProvider`.

## Feature flags

| Flag | Enables | Dependency |
|---|---|---|
| `rhai-runtime` | Rhai script executor | `rhai` |
| `lua-runtime` | Lua script executor | `mlua` |
| `wasm-runtime` | WASM executor | `extism` |
| `sandboxed` | Process sandboxing for external runtimes | `synwire-sandbox` |

The `external` and `sequence` runtimes are always available regardless of feature flags.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | `ToolProvider`, `SamplingProvider` traits |
| `synwire-storage` | `StorageLayout` for skill directory resolution |
| `serde_yaml` | YAML frontmatter parsing |
| `walkdir` / `globset` | Directory scanning and pattern matching |

## Ecosystem position

Skills are registered as MCP tools by `synwire-mcp-server` at startup. The server scans the global skills directory, loads each skill via `SkillLoader`, and wraps it as a tool in the `ToolSearchIndex`. When an LLM activates a skill, the server dispatches to the appropriate `SkillExecutor`.

## See also

- [Authoring Your First Agent Skill](../tutorials/12-first-skill.md) --- tutorial
- [synwire-mcp-server](./synwire-mcp-server.md) --- where skills become MCP tools
- [synwire-core: Trait Contract Layer](./synwire-core.md) --- `ToolProvider` and `SamplingProvider`
