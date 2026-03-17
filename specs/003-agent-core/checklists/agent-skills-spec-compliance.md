# Agent Skills Spec Compliance Checklist: Agent Core Runtime

**Purpose**: Validate requirements quality, completeness, and consistency for agentskills.io spec compliance and embedded runtime support (US33, FR-799–803x)
**Created**: 2026-03-16
**Feature**: [spec.md](../spec.md) — US33, FR-799–803x, SC-109–109d

## Requirement Completeness — agentskills.io Conformance

- [x] CHK001 - Are all six standard frontmatter fields explicitly specified with their exact constraints? [Completeness, Spec §FR-803c]
  > ✅ FR-803c: name (1-64 chars, lowercase+hyphens, match dir), description (1-1024 chars), license, compatibility (1-500 chars), metadata (string→string), allowed-tools (space-delimited).
- [x] CHK002 - Are all `name` field validation rules documented? [Completeness, Spec §FR-803c]
  > ✅ FR-803c: "1-64 chars, lowercase alphanumeric + hyphens, no leading/trailing/consecutive hyphens, must match directory name"
- [x] CHK003 - Is the `description` field constraint and keyword guidance documented? [Completeness, Spec §FR-803c]
  > ✅ FR-803c: "1-1024 chars". agentskills.io spec guidance about keywords is in the standard, referenced by FR-803a.
- [x] CHK004 - Is the `metadata` field specified as string→string map? [Clarity, Spec §FR-803c]
  > ✅ FR-803c: "metadata (string→string map for author/version/etc.)" — matches agentskills.io.
- [x] CHK005 - Is `allowed-tools` specified as space-delimited? [Clarity, Spec §FR-803c]
  > ✅ FR-803c: "allowed-tools (optional, experimental, space-delimited tool list)" — matches agentskills.io format.
- [x] CHK006 - Are all three progressive disclosure phases defined with token budgets? [Completeness, Spec §FR-803e]
  > ✅ FR-803e: "metadata ~100 tokens at startup, full body <5000 tokens on activation, files on demand"
- [x] CHK007 - Are the standard directory conventions documented? [Completeness, Spec §FR-803b]
  > ✅ FR-803b: "SKILL.md + optional scripts/, references/, assets/ subdirectories"
- [x] CHK008 - Is the SKILL.md body content guidance documented? [Gap]
  > ✅ FR-803m: "under 500 lines, detailed reference to references/ files, relative paths one level deep"
- [x] CHK009 - Are file reference conventions documented? [Gap]
  > ✅ FR-803m: "file references MUST use relative paths from skill root, kept one level deep"

## Requirement Completeness — Synwire Extensions

- [x] CHK010 - Is the `runtime` extension clearly documented as synwire-specific? [Clarity, Spec §FR-803d]
  > ✅ FR-803d: "synwire MUST extend the standard frontmatter with an optional `runtime` field"
- [x] CHK011 - Are all five runtime values documented with execution semantics? [Completeness, Spec §FR-803d]
  > ✅ FR-803d (values), FR-803f (Lua), FR-803g (Rhai), FR-803h (WASM), FR-803r (tool-sequence), FR-803l (external).
- [x] CHK012 - Is the default behaviour when `runtime` is absent specified? [Completeness, Spec §FR-803d]
  > ✅ FR-803d: "When runtime is absent, scripts are executed as external subprocesses per the standard Agent Skills model"
- [x] CHK013 - Are VFS host function bindings enumerated? [Gap, Spec §FR-803f–803h]
  > ✅ FR-803n: minimum set enumerated (read, grep, glob, tree, head, stat). Write ops conditional on allowed-tools.
- [x] CHK014 - Is the `CreateTool` auto-generated manifest format specified? [Gap, Spec §FR-801]
  > ✅ FR-803x: name from tool name, description from directive, runtime from script language, script to scripts/. Written to $DATA/<product>/skills/.

## Requirement Clarity

- [x] CHK015 - Is Lua instruction count limit quantified? [Clarity, Spec §FR-803f]
  > ✅ FR-803o: "default 1,000,000 instructions, configurable per-skill via metadata.max_operations"
- [x] CHK016 - Is Rhai max_operations quantified? [Clarity, Spec §FR-803g]
  > ✅ FR-803o: "default 1,000,000, configurable per-skill via metadata.max_operations"
- [x] CHK017 - Is "source code preserved alongside .wasm" precise? [Ambiguity, Spec §FR-803h]
  > ✅ FR-803h: "MUST preserve the original source code alongside the compiled .wasm module (e.g., scripts/plugin.rs + scripts/plugin.wasm), enabling auditability, recompilation, and modification"
- [x] CHK018 - Is "project-local precedence over global" defined for same name/different version? [Ambiguity, Spec §FR-803i]
  > ✅ FR-803i: "Project-local skills take precedence over global skills with the same name" — regardless of version. Local always wins.
- [x] CHK019 - Is version comparison semver or lexicographic? [Ambiguity, Spec §FR-803k]
  > ✅ FR-803t: "semantic versioning (semver) — major.minor.patch"
- [x] CHK020 - Is the external runtime warning specific enough? [Clarity, Spec §FR-803l]
  > ✅ FR-803l + edge case: "emit warning at load time: 'runtime external bypasses embedded sandboxing — prefer lua, rhai, or wasm'. Logged and surfaced to user on first invocation."

## Requirement Consistency

- [x] CHK021 - Is skill `allowed-tools` consistent with agent-level `allowed_tools`/`excluded_tools`? [Consistency]
  > ✅ Independent mechanisms. Agent-level allowed_tools (FR-605) controls which agent tools are available. Skill allowed-tools (FR-803c) controls which VFS ops the skill's runtime can access. Both are checked.
- [x] CHK022 - Are skill sandbox permissions consistent with `SandboxConfig`? [Consistency]
  > ✅ FR-803j: "check that requested permissions are compatible with agent's SandboxConfig and PermissionMode before registering". Skill can't exceed agent's sandbox.
- [x] CHK023 - Is the skill discovery path consistent with StorageLayout? [Consistency, Spec §FR-803i]
  > ✅ FR-803i references StorageLayout.skills_dir() and project_skills_dirname(). Consistent.
- [x] CHK024 - Are CreateTool directive permissions consistent with native tools? [Consistency]
  > ✅ FR-800: "Dynamically-created tools MUST pass through the same sandbox, permission, and approval checks as native tools"
- [x] CHK025 - Is skills auto-loading in MCP server consistent with SkillLoader? [Consistency]
  > ✅ FR-888i: startup only. FR-803w: confirms no hot-reload. FR-803i/803j: loader validates and registers. Consistent.

## Acceptance Criteria Quality

- [x] CHK026 - Is SC-109a testable? [Measurability, Spec §SC-109a]
  > ✅ Test: create Lua and Rhai skills that grep+format, call both with same input, assert identical output. Measurable.
- [x] CHK027 - Is SC-109b testable? [Measurability, Spec §SC-109b]
  > ✅ FR-803p: WASM calling unlisted host function gets permission denied. Test: WASM skill without vfs.write in allowed-tools tries write, assert error. Measurable.
- [x] CHK028 - Is SC-109d defined with specific failures? [Measurability, Spec §SC-109d]
  > ✅ FR-803j: validates name format, description length, directory name match, runtime availability, entrypoint existence. Each is a distinct failure. Testable.

## Scenario Coverage

- [x] CHK029 - Are requirements for skill discovery ordering defined? [Coverage, Gap]
  > ✅ FR-803s: "filesystem order within each directory. No explicit priority beyond project-local > global."
- [x] CHK030 - Are requirements for hot-reloading defined? [Coverage, Gap]
  > ✅ FR-803w: "NOT supported. Load at startup. Restart to add/modify."
- [x] CHK031 - Are requirements for skill unloading/removal defined? [Coverage, Gap]
  > ✅ Implicit: skills are loaded at startup, session-scoped. To remove, delete the skill directory and restart. No explicit unload API needed — acceptable for a file-based system.
- [x] CHK032 - Are requirements for skill dependencies defined? [Coverage]
  > ✅ FR-803b mentions optional dependencies. Implementation: dependency skills are loaded first. If a dependency is missing, the dependent skill fails validation at load time. Low complexity — acceptable level of specification.
- [x] CHK033 - Are requirements for tool-sequence runtime defined? [Gap, Spec §FR-801]
  > ✅ FR-803r: JSON array of tool invocations in scripts/sequence.json. `$result[N]` for step chaining.
- [x] CHK034 - Are requirements for how skill errors are surfaced? [Coverage, Gap]
  > ✅ FR-803q: "returned as tool errors (ToolOutput with status: Failure), including skill name and error message"

## Edge Case Coverage

- [x] CHK035 - Is behaviour defined for SKILL.md with valid frontmatter but empty body? [Edge Case, Gap]
  > ✅ Valid per agentskills.io spec — no body restrictions. The skill loads but has no instructions. Activation loads an empty body. Acceptable — the agent sees name+description, calls the skill, gets no guidance. This is a poorly-written skill, not an error.
- [x] CHK036 - Is behaviour defined for unsupported script languages in scripts/? [Edge Case, Gap]
  > ✅ FR-803j: "check that the declared runtime is available". If runtime doesn't support the script language, validation fails at load time. Scripts without a runtime field use external execution.
- [x] CHK037 - Is behaviour defined when Lua/Rhai scripts attempt imports? [Edge Case, Gap]
  > ✅ FR-803u: "Rhai MUST NOT import external modules. Lua MUST have require disabled (sandbox mode)."
- [x] CHK038 - Is behaviour defined when WASM exceeds memory? [Edge Case, Gap]
  > ✅ FR-803v: "bounded by Extism default, configurable via metadata.max_memory_mb (default 64MB). Exceeding = OOM error."
- [x] CHK039 - Is behaviour defined when metadata.version is absent? [Edge Case, Gap]
  > ✅ FR-803t: "treated as version 0.0.0 for precedence comparisons"

## Dependencies & Assumptions

- [x] CHK040 - Is the mlua LuaJIT/Lua 5.4 choice documented? [Assumption, Spec §FR-803f]
  > ✅ FR-803f: "mlua (LuaJIT or Lua 5.4)". Implementation chooses at build time. Both supported by mlua feature flags. Acceptable.
- [x] CHK041 - Is the Extism SDK version documented? [Dependency, Gap]
  > ✅ Implementation detail — the Cargo.toml will pin the version. The spec references Extism PDK host functions, which is the stable API. Acceptable.
- [x] CHK042 - Is the YAML spec version for frontmatter documented? [Dependency, Gap]
  > ✅ serde_yaml uses YAML 1.2 by default. agentskills.io doesn't pin a YAML version. The frontmatter is simple key-value — no YAML version ambiguity for this use case. Acceptable.
- [x] CHK043 - Is the agentskills.io spec stability validated? [Assumption]
  > ✅ FR-803a: "implementing the Agent Skills specification (agentskills.io/specification)". The spec is published and used by Claude Code. Stable enough for implementation.

## Notes

- Check items off as completed: `[x]`
- All 43 items resolved. 12 new FRs (FR-803m–803x) added to spec.
- This checklist covers both agentskills.io conformance AND synwire runtime extensions
