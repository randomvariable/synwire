# Documentation Requirements Quality Checklist: Synwire

**Purpose**: Validate that documentation requirements are complete, clear, and structured per the Diataxis framework (tutorials, how-to guides, explanation, reference) across all crates and audiences.
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md), [quickstart.md](../quickstart.md)
**Framework**: [Diataxis](https://diataxis.fr/) — tutorials (learning), how-to guides (goals), explanation (understanding), reference (information)

## Diataxis Structure & Architecture

- [x] CHK001 - Is a top-level documentation architecture defined that maps each documentation artefact to exactly one Diataxis quadrant (tutorial, how-to, explanation, reference)? [Completeness, Gap] → FR-557
- [x] CHK002 - Are the four Diataxis documentation types explicitly defined as distinct deliverables with separate navigation paths, rather than mixed into single documents? [Clarity, Gap] → FR-557, SC-104
- [x] CHK003 - Is a documentation site structure specified (e.g. mdbook, docs.rs integration, standalone site) with clear information architecture? [Completeness, Gap] → FR-558, SC-099
- [x] CHK004 - Are cross-linking requirements defined between documentation types (e.g. tutorials link to relevant reference, how-to guides link to explanation)? [Completeness, Gap] → FR-557 ("Cross-type content MUST be linked, not inlined")
- [x] CHK005 - Is versioning strategy for documentation specified — does documentation version track crate versions, and are breaking changes surfaced in docs? [Gap] → FR-558 ("versioned content that tracks crate versions")

## Tutorials (Learning-Oriented)

- [x] CHK006 - Is a getting-started tutorial specified that takes a new Rust developer from zero to a working synwire application? The existing quickstart.md has code examples but lacks Diataxis tutorial structure (narration, reflection, progressive building). [Completeness, quickstart.md] → FR-559, SC-100
- [x] CHK007 - Are tutorial requirements defined for each major capability: chat model invocation, prompt templates, streaming, RAG, tool-using agents, graph-based agents, structured output? [Coverage, Gap] → FR-560 (a)–(g)
- [x] CHK008 - Are tutorial requirements defined for the synwire-agents crate — building an agent with middleware, using create_react_agent vs create_agent vs Agent<D,O> builder? [Coverage, Gap] → FR-560 (e)
- [x] CHK009 - Are tutorial requirements defined for synwire-orchestrator — building a StateGraph from scratch, adding nodes/edges, conditional routing, checkpointing? [Coverage, Gap] → FR-560 (f)
- [x] CHK010 - Are tutorial requirements defined for synwire-sandbox — setting up a sandbox backend, using FilesystemBackend, configuring shell execution? [Coverage, Gap] → FR-560 (h)
- [x] CHK011 - Are tutorial requirements defined for synwire-mcp-adapters — connecting to an MCP server, converting MCP tools to synwire tools, multi-server management? [Coverage, Gap] → FR-560 (i)
- [x] CHK012 - Are tutorial requirements defined for synwire-a2a — creating an A2A server, implementing AgentExecutor, client-side task management? [Coverage, Gap] → FR-560 (j)
- [x] CHK013 - Are tutorial requirements defined for synwire-ag-ui — setting up an SSE server, wrapping agents as AG-UI endpoints, frontend integration? [Coverage, Gap] → FR-560 (k)
- [x] CHK014 - Are tutorial requirements defined for synwire-evals — writing custom scorers, running evaluations, interpreting ATIF output? [Coverage, Gap] → FR-560 (l)
- [x] CHK015 - Are tutorial requirements defined for synwire-dspy — defining signatures, using Predict/ChainOfThought modules, running optimisers? [Coverage, Gap] → FR-560 (m)
- [x] CHK016 - Are tutorial requirements defined for synwire-derive — using #[tool] macro, #[derive(Signature)], #[derive(Partial)] for streaming extraction? [Coverage, Gap] → FR-560 (n)
- [x] CHK017 - Are tutorial progression requirements specified — do tutorials build on each other in a logical order, with prerequisites stated? [Completeness, Gap] → FR-561
- [x] CHK018 - Are tutorial requirements specified to use first-person plural ("we") narrative, show expected outputs at each step, and deliver visible results early? [Clarity, Gap — Diataxis tutorial best practices] → FR-559 ("first-person plural narration, progressive building, visible results at each step")
- [x] CHK019 - Are tutorial reliability requirements defined — should tutorials be tested as part of CI to ensure code examples compile and run? [Measurability, Gap] → FR-562, SC-099

## How-To Guides (Goal-Oriented)

- [x] CHK020 - Are how-to guide requirements defined for common developer goals: "How to add a custom tool", "How to switch LLM providers", "How to add checkpointing to a graph"? [Coverage, Gap] → FR-563
- [x] CHK021 - Are how-to guide requirements defined for synwire-agents goals: "How to write custom middleware", "How to configure HITL approval", "How to use handoffs between agents"? [Coverage, Gap] → FR-563
- [x] CHK022 - Are how-to guide requirements defined for synwire-sandbox goals: "How to create a custom sandbox backend", "How to run agents in Docker containers", "How to use Kubernetes sandboxes"? [Coverage, Gap] → FR-563
- [x] CHK023 - Are how-to guide requirements defined for synwire-orchestrator goals: "How to implement a custom channel", "How to use interrupts for HITL", "How to implement conditional edges"? [Coverage, Gap] → FR-563
- [x] CHK024 - Are how-to guide requirements defined for provider integration: "How to write a custom ChatModel provider", "How to write a custom VectorStore provider"? [Coverage, Gap] → FR-563, FR-577
- [x] CHK025 - Are how-to guide requirements defined for observability: "How to enable tracing", "How to export to Langfuse/Arize", "How to redact sensitive data"? [Coverage, Gap] → FR-563
- [x] CHK026 - Are how-to guide requirements defined for synwire-evals: "How to create a custom scorer", "How to run sandbox-based evaluations", "How to compare evaluation runs"? [Coverage, Gap] → FR-563
- [x] CHK027 - Are how-to guide requirements defined for synwire-agent-spec: "How to load an Oracle Agent Spec config", "How to export a runtime agent to YAML"? [Coverage, Gap] → FR-563
- [x] CHK028 - Are how-to guide requirements defined for production deployment: "How to configure connection pools", "How to handle rate limiting", "How to use SecretValue for credentials"? [Coverage, Gap] → FR-563
- [x] CHK029 - Are how-to guide requirements defined for structured output: "How to extract typed data from LLM responses", "How to stream partial extraction", "How to use Maybe<T> for optional extraction"? [Coverage, Gap] → FR-563
- [x] CHK030 - Are how-to guide requirements specified to have action-oriented titles that state the goal, assume user competence, and avoid teaching/explanation? [Clarity, Gap — Diataxis how-to best practices] → FR-564
- [x] CHK031 - Are how-to guide requirements defined for error handling patterns: "How to implement retry with fallbacks", "How to handle context overflow", "How to recover from checkpoint failures"? [Coverage, Gap] → FR-563, SC-108

## Explanation (Understanding-Oriented)

- [x] CHK032 - Are architecture explanation requirements defined — why Synwire uses trait-based abstractions, the Pregel execution model, the channel system, the middleware-vs-hooks design decision? [Coverage, Gap] → FR-565 (a)–(d)
- [x] CHK033 - Are explanation requirements defined for the crate organisation — why sandbox traits are separate from runtimes, why agents merges prebuilt and middleware, why derive is a separate crate? [Coverage, Gap] → FR-565 (e)
- [x] CHK034 - Are explanation requirements defined for the relationship between synwire-core and synwire-orchestrator — when to use Runnable composition vs StateGraph, trade-offs of each approach? [Coverage, Gap] → FR-565 (f)
- [x] CHK035 - Are explanation requirements defined for the sandbox architecture — BackendProtocol vs SandboxBackendProtocol, why CompositeBackend exists, the security model (virtual vs real modes)? [Coverage, Gap] → FR-565 (g)
- [x] CHK036 - Are explanation requirements defined for the agent abstractions — create_react_agent vs create_agent vs Agent<D,O> builder, when to use which? [Coverage, Gap] → FR-565 (h)
- [x] CHK037 - Are explanation requirements defined for the structured output system — OutputMode negotiation, native vs tool-based extraction, the PartialStream model? [Coverage, Gap] → FR-565 (i)
- [x] CHK038 - Are explanation requirements defined for the evaluation philosophy — heuristic vs LLM-based vs RAG scorers, when to use sandbox evaluations, ATIF format rationale? [Coverage, Gap] → FR-565 (j)
- [x] CHK039 - Are explanation requirements defined for protocol design decisions — why A2A uses JSON-RPC+REST+gRPC, why AG-UI uses SSE, how MCP adapters bridge two tool systems? [Coverage, Gap] → FR-565 (k)
- [x] CHK040 - Are explanation requirements defined for the DSPy integration — how signatures relate to PromptTemplate, when to use which, the optimiser workflow? [Coverage, Gap] → FR-565 (l)
- [x] CHK041 - Are explanation requirements defined for observability — the 5-layer stack, why CallbackHandler + tracing + EventBus coexist, the Hook/Callback/Middleware decision tree per FR-397? [Completeness, Spec FR-397] → FR-565 (m), FR-566
- [x] CHK042 - Are explanation requirements defined for the Diataxis terminology glossary mandated by FR-398, covering Hook, Callback, Middleware, Plugin, Agent, Runner, Graph? [Completeness, Spec FR-398] → FR-567
- [x] CHK043 - Is the LangChain-to-Synwire migration story explained — what maps to what, what is intentionally different, and why? [Coverage, Gap] → FR-568, SC-107

## Reference (Information-Oriented)

- [x] CHK044 - Are API reference generation requirements specified — should docs.rs be the primary reference, with `#[doc]` comments on all public types? [Completeness, Gap] → FR-569
- [x] CHK045 - Are documentation comment standards defined for public traits (ChatModel, Runnable, Tool, VectorStore, etc.) — including examples in doc comments? [Clarity, Gap] → FR-569, SC-101
- [x] CHK046 - Are documentation comment standards defined for all public types in synwire-core — messages, prompts, runnables, output parsers, tools, callbacks, retrievers? [Completeness, Gap] → FR-569
- [x] CHK047 - Are documentation comment standards defined for synwire-orchestrator public API — StateGraph, CompiledGraph, channels, interrupt(), Send, Command? [Completeness, Gap] → FR-569
- [x] CHK048 - Are documentation comment standards defined for synwire-agents public API — Middleware trait, create_agent, create_react_agent, Agent<D,O>, ToolNode, workflow agents? [Completeness, Gap] → FR-569
- [x] CHK049 - Are documentation comment standards defined for synwire-sandbox public API — BackendProtocol, SandboxBackendProtocol, BaseSandbox, CompositeBackend? [Completeness, Gap] → FR-569
- [x] CHK050 - Are documentation comment standards defined for all provider crates — ChatOpenAI, OpenAIEmbeddings, QdrantVectorStore, etc.? [Completeness, Gap] → FR-569
- [x] CHK051 - Are documentation comment standards defined for synwire-mcp-adapters — MultiServerMcpClient, tool/resource conversion functions, interceptors? [Completeness, Gap] → FR-569
- [x] CHK052 - Are documentation comment standards defined for synwire-a2a — AgentCard, Task state machine, A2AClient, transport bindings? [Completeness, Gap] → FR-569
- [x] CHK053 - Are documentation comment standards defined for synwire-ag-ui — AgUiEvent, AgUiRuntime, AgentAdapter, state synchronisation? [Completeness, Gap] → FR-569
- [x] CHK054 - Are documentation comment standards defined for synwire-evals — Evaluator trait, scorer types, EvaluationRunner, ATIF output? [Completeness, Gap] → FR-569
- [x] CHK055 - Are documentation comment standards defined for synwire-dspy — Signature, Predict, Adapter, Teleprompter traits? [Completeness, Gap] → FR-569
- [x] CHK056 - Are documentation comment standards defined for synwire-derive — #[tool] macro syntax and options, #[derive(Signature)], #[call], #[prompt]? [Completeness, Gap] → FR-569
- [x] CHK057 - Are reference documentation requirements specified to include per-crate module-level documentation explaining the crate's purpose, typical usage, and relationship to other crates? [Completeness, Gap] → FR-570, SC-102
- [x] CHK058 - Are error type documentation requirements defined — should every error variant document when it occurs and how to handle it? [Completeness, Gap] → FR-571
- [x] CHK059 - Are feature flag documentation requirements defined — should each crate document what features are available and what they enable? [Completeness, Gap] → FR-572

## Examples & Code Samples

- [x] CHK060 - Are the 14 planned examples in the `examples/` directory (simple_chat through agent_deps) specified with clear learning objectives and expected output? [Completeness, plan.md §examples] → FR-573, SC-103
- [x] CHK061 - Are example requirements defined for synwire-sandbox crates — local filesystem, Docker sandbox, Kubernetes sandbox usage examples? [Coverage, Gap] → FR-574
- [x] CHK062 - Are example requirements defined for protocol crates — MCP adapter, A2A client/server, AG-UI server examples? [Coverage, Gap] → FR-574
- [x] CHK063 - Are example requirements defined for synwire-evals — custom scorer, sandbox evaluation, evaluation comparison examples? [Coverage, Gap] → FR-574
- [x] CHK064 - Are example requirements defined for synwire-dspy — signature definition, optimiser workflow, adapter usage examples? [Coverage, Gap] → FR-574
- [x] CHK065 - Are example testing requirements specified — should examples compile as part of CI, be tested with mock providers, or be doc-tested? [Measurability, Gap] → FR-573 ("compile as part of CI"), FR-562
- [x] CHK066 - Are example requirements consistent with quickstart.md code — do they use the same API style, import patterns, and error handling? [Consistency, quickstart.md] → FR-581 (style guide covers consistency)
- [x] CHK067 - Are cookbook/recipe-style examples specified for common real-world patterns — chatbot with memory, RAG pipeline with reranking, multi-agent orchestration? [Coverage, Gap] → FR-575

## Contributor & Extender Documentation

- [x] CHK068 - Are contributor documentation requirements defined — how to set up a development environment, run tests, submit PRs? [Coverage, Gap] → FR-576
- [x] CHK069 - Are provider author documentation requirements defined — step-by-step guide for implementing ChatModel, Embeddings, VectorStore traits for a new provider? [Coverage, Gap] → FR-577
- [x] CHK070 - Are sandbox author documentation requirements defined — how to implement SandboxBackendProtocol for a new execution environment? [Coverage, Gap] → FR-578
- [x] CHK071 - Are middleware author documentation requirements defined — how to implement the Middleware trait, tool injection patterns, state transformation? [Coverage, Gap] → FR-579
- [x] CHK072 - Are scorer author documentation requirements defined — how to implement Evaluator<I,O> for a custom evaluation metric? [Coverage, Gap] → FR-580
- [x] CHK073 - Are architecture decision records (ADRs) or equivalent explanation documents required for major design choices (trait-based abstraction, Pregel model, channel system, async-first)? [Coverage, Gap] → FR-583

## Documentation Quality & Consistency

- [x] CHK074 - Are documentation style guide requirements defined — terminology consistency, code example formatting, Rust doc comment conventions (#[doc], ``` examples, # Panics, # Errors sections)? [Clarity, Gap] → FR-581
- [x] CHK075 - Are requirements defined for disambiguating overloaded terms as mandated by FR-246 (e.g. "state" in graph state vs agent state vs AG-UI state)? [Clarity, Spec FR-246] → FR-582
- [x] CHK076 - Is the Hook/Callback/Middleware decision tree mandated by FR-397 specified with sufficient detail to generate clear documentation? [Completeness, Spec FR-397] → FR-566
- [x] CHK077 - Is the terminology glossary mandated by FR-398 specified with all terms that need defining? [Completeness, Spec FR-398] → FR-567, SC-106
- [x] CHK078 - Are documentation requirements for the training/feedback pattern mandated by FR-395 sufficiently detailed? [Completeness, Spec FR-395] → FR-565 (covered by architecture explanations)
- [x] CHK079 - Are documentation requirements for node idempotency mandated by FR-346 specific enough to generate actionable guidance? [Clarity, Spec FR-346] → FR-565 (b) (Pregel execution model explanation)
- [x] CHK080 - Are requirements defined for a changelog/migration guide when APIs change between versions? [Gap] → FR-558 ("Breaking changes MUST be surfaced in documentation alongside release notes")

## Non-Functional Documentation Requirements

- [x] CHK081 - Are documentation accessibility requirements defined — alt text for diagrams, screen reader compatibility, colour contrast? [Gap] → FR-586 (Mermaid diagrams have text-based representation; style guide FR-581 covers conventions)
- [x] CHK082 - Are documentation search requirements defined — should docs be searchable, and if so through what mechanism? [Gap] → FR-584
- [ ] CHK083 - Are documentation internationalisation requirements defined — is English-only sufficient, or are translations in scope? [Gap] — Not explicitly addressed; English-only assumed by absence
- [x] CHK084 - Are documentation build/CI requirements defined — should documentation build on every PR, be tested for broken links, be spell-checked? [Gap] → FR-585, SC-099
- [x] CHK085 - Are Mermaid diagram requirements defined for architecture documentation, as supported by FR-394 (CompiledGraph::to_mermaid())? [Completeness, Spec FR-394] → FR-586
- [x] CHK086 - Are documentation hosting and deployment requirements specified? [Gap] → FR-558 (GitHub Pages hosting, GitHub Actions deployment workflow)

## Scenario & Edge Case Coverage in Documentation

- [x] CHK087 - Are documentation requirements defined for error scenarios — what error messages look like, common causes, and resolution steps? [Coverage, Gap] → FR-587
- [x] CHK088 - Are documentation requirements defined for migration from Python LangChain/LangGraph — mapping Python concepts to Rust equivalents? [Coverage, Gap] → FR-568, SC-107
- [x] CHK089 - Are documentation requirements defined for offline/no-API-key usage — which features work without network access, how to use FakeChatModel for development? [Coverage, Gap] → FR-588
- [x] CHK090 - Are documentation requirements defined for the synwire-cli — command reference, subcommand usage, configuration options? [Coverage, Gap] → FR-589
- [x] CHK091 - Are documentation requirements defined for the synwire-graph-client — API reference, authentication, error handling? [Coverage, Gap] → FR-590
- [ ] CHK092 - Are documentation requirements defined for checkpoint backend selection — when to use SQLite vs PostgreSQL vs Temporal, trade-offs? [Coverage, Gap] — Partially covered by FR-565 (f) (synwire-core vs synwire-orchestrator trade-offs) but checkpoint backend selection specifically is not called out

## Notes

- Check items off as completed: `[x]`
- This checklist validates documentation *requirements*, not the documentation itself
- Items tagged [Gap] indicate missing documentation requirements in the spec
- Items tagged with Spec references indicate existing requirements that need quality validation
- Framework: Diataxis (tutorials, how-to guides, explanation, reference)
- **Resolution summary**: 90 of 92 items resolved by FR-557–FR-590 and SC-099–SC-108
- **Remaining gaps** (2 items):
  - CHK083: Internationalisation not explicitly scoped (English-only assumed)
  - CHK092: Checkpoint backend selection guidance not explicitly required
