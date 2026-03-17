# Feature Specification: Agent Core Runtime

**Feature Branch**: `003-agent-core`
**Created**: 2026-03-15
**Status**: Draft (expanded 2026-03-16, clarified 2026-03-16, MCP adapters scope added 2026-03-16)
**Input**: User description: "Minimum viable agent runtime with directives, execution strategies, plugin system, signal routing, backends, middleware, and streaming events" + VFS refactor, semantic search pipeline (chunker, embeddings, vector store, index), LSP/DAP integration, process sandboxing, and research-driven code localization improvements (per-method chunking, file skeletons, hierarchical narrowing, code graphs, hybrid search, SBFL, repository memory, MCTS, self-evolving tools, dataflow retrieval)

## Clarifications

### Session 2026-03-16

- Q: What transport modes should the MCP server binary support, and what is the security/authentication model? → A: Stdio transport only (no HTTP). Multiple concurrent instances supported across code editor instances (e.g., two Claude Code windows, one Copilot), sharing persistent data safely via `ProjectLock`.
- Q: What observability should the MCP server and indexing pipeline provide? → A: Both stderr and file logging. Structured `tracing` logs to stderr (info level default, configurable via `RUST_LOG` / `--log-level`), plus rotated log files under `StorageLayout` (`$DATA/<product>/logs/`) for post-mortem analysis.
- Q: What is the maximum target codebase size for indexing, code graph, and search? → A: Must handle very large repos including the Linux kernel (~70,000 files, ~30M LOC). Index, code graph, and community state must be disk-backed and streaming, not fully in-memory.
- Q: Should original FRs (FR-070–081) be updated to use current VFS names or left as historical record? → A: Update to current names. The spec is a living document, not a changelog.
- Q: Which embedding model should be the default for semantic search? → A: bge-small-en-v1.5 (33MB, fast) as default, configurable to bge-base or bge-large via `--embedding-model` flag or config. Reranker compensates for smaller embedding model quality.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pure Testable Agent Logic (Priority: P1)

Developers can write agent decision logic as pure functions that are fully unit-testable without spawning processes, mocking filesystem calls, or stubbing network connections. Agent functions return state changes and typed effect descriptions, enabling developers to verify agent decisions without executing side effects.

**Why this priority**: Core capability that enables test-driven development and confidence in agent behavior. Without this, agent testing requires complex integration setup.

**Independent Test**: Can be fully tested by writing an agent node that returns directives, asserting the returned directives match expectations, and confirming no actual side effects occurred (no files written, no processes spawned).

**Acceptance Scenarios**:

1. **Given** an agent node implementation, **When** the node receives input state, **Then** it returns updated state and zero or more typed effect descriptions without executing any side effects
2. **Given** an agent returning a "spawn subagent" directive, **When** the test executor records directives, **Then** no actual subagent process is created and the directive is captured for verification
3. **Given** an agent test suite, **When** running unit tests, **Then** all agent logic is testable without filesystem access, network calls, or subprocess execution

---

### User Story 2 - Pluggable Execution Strategies (Priority: P1)

Developers can run identical agent logic under different execution strategies (immediate sequential execution vs. finite state machine with transition guards) by configuration alone, without rewriting agent code. The same agent can enforce "action X only valid in state Y" constraints when needed, or run unrestricted for simple workflows.

**Why this priority**: Enables the same agent logic to support both simple request-response flows and complex multi-step workflows with state constraints. Critical for reusability.

**Independent Test**: Can be fully tested by running the same agent logic first with immediate execution strategy (succeeds on any action sequence) and then with FSM strategy (enforces state transition rules), verifying both produce correct results for valid sequences and FSM correctly rejects invalid transitions.

**Acceptance Scenarios**:

1. **Given** an agent and an immediate execution strategy, **When** actions are submitted in any order, **Then** the agent executes each action immediately without state validation
2. **Given** the same agent with an FSM execution strategy defining valid state transitions, **When** an action invalid for the current state is submitted, **Then** the system rejects it with an explicit "invalid transition" error including current state and attempted action
3. **Given** the same agent logic under both strategies, **When** provided identical valid action sequences, **Then** both strategies produce the same final state and results

---

### User Story 3 - Composable Plugin System with State Isolation (Priority: P1)

Developers can compose multiple plugins into an agent runtime where each plugin manages its own state slice without interference from other plugins. Plugin state is type-safe, preventing one plugin from accidentally reading or modifying another plugin's data.

**Why this priority**: Enables building complex agent systems from reusable, independently-developed plugins without namespace collisions or state corruption.

**Independent Test**: Can be fully tested by composing two plugins with different state types, having each plugin write to its state concurrently, then verifying each plugin can only access its own state and the states remain isolated.

**Acceptance Scenarios**:

1. **Given** two plugins with different state types, **When** both plugins are composed into an agent, **Then** each plugin can access only its own state slice via type-safe accessors
2. **Given** plugins A and B writing to their respective states, **When** plugin A attempts to read plugin B's state without proper access, **Then** the compiler prevents this at build time via type system
3. **Given** multiple plugins composed together, **When** their state key names conflict, **Then** the system produces a compile-time error preventing runtime state corruption

---

### User Story 4 - File and Shell Operations via Backend Protocol (Priority: P2)

Developers can write agents that perform file operations (read, write, edit, search, upload, download) and shell execution through a uniform interface with bash-style command conventions (ls, cd, grep, rm, cp, mv). Backend implementations support ephemeral in-memory storage, persistent cross-conversation storage, real filesystem access, or composite routing to multiple backends.

**Why this priority**: Essential for practical agent applications that need to interact with files and execute commands. Bash-style conventions provide familiar interface for developers. Enables testability by swapping backends.

**Independent Test**: Can be fully tested by creating an agent that performs file operations and bash-style commands against an ephemeral in-memory backend, verifying operations succeed, then swapping to a persistent backend and confirming cross-conversation file retention.

**Acceptance Scenarios**:

1. **Given** an agent with an ephemeral backend, **When** the agent writes files during a conversation, **Then** files are accessible within that conversation but disappear after conversation ends
2. **Given** an agent with a persistent backend, **When** the agent writes files in conversation A, **Then** those files are accessible in subsequent conversation B
3. **Given** an agent with a composite backend routing `/tmp/` to ephemeral storage and `/persistent/` to persistent storage, **When** files are written to both paths, **Then** each file is routed to the correct backend based on path prefix
4. **Given** an agent with a filesystem backend, **When** file operations are attempted with paths containing `..` traversal sequences, **Then** the backend rejects the operation with a path traversal error
5. **Given** an agent with bash-style command support, **When** using commands like ls, cd, grep, rm, cp, mv, **Then** the backend translates these to appropriate file operations maintaining familiar shell semantics

---

### User Story 5 - Middleware Stack for Cross-Cutting Concerns (Priority: P2)

Developers can augment agent capabilities by composing middleware components that add tools, modify system prompts, or transform state. Middleware executes in a defined order and can inspect or modify agent inputs and outputs without changing agent core logic.

**Why this priority**: Enables separation of concerns and reusable cross-cutting functionality like caching, summarization, and tool injection.

**Independent Test**: Can be fully tested by building an agent with and without specific middleware (e.g., prompt caching middleware), verifying the middleware correctly transforms inputs/outputs, and confirming agent logic remains unchanged.

**Acceptance Scenarios**:

1. **Given** an agent with filesystem middleware, **When** the agent is initialized, **Then** file operation tools are automatically available without manual tool registration
2. **Given** an agent with summarization middleware configured with a message count threshold, **When** message count exceeds the threshold, **Then** the middleware automatically triggers context summarization
3. **Given** an agent with middleware in a defined stack order, **When** processing a request, **Then** each middleware executes in the declared order and can transform state or add tools for subsequent middleware

---

### User Story 6 - Three-Tier Signal Routing (Priority: P2)

Developers can control how incoming signals (messages, events, interrupts) are routed to agent actions through three priority levels: execution strategy routes (highest), agent-level routes (middle), and plugin-contributed routes (lowest), with first match winning.

**Why this priority**: Enables execution strategies to gate signals based on state (e.g., reject user input during processing), while allowing agents and plugins to define default routing.

**Independent Test**: Can be fully tested by defining conflicting routes at all three levels for the same signal, sending the signal, and verifying the strategy-level route is selected over agent and plugin routes.

**Acceptance Scenarios**:

1. **Given** an execution strategy that defines a route for signal X, **When** signal X arrives, **Then** the strategy route is selected even if agent and plugin routes for X exist
2. **Given** no strategy route for signal Y but an agent-level route exists, **When** signal Y arrives, **Then** the agent route is selected over plugin routes
3. **Given** no strategy or agent routes for signal Z but a plugin route exists, **When** signal Z arrives, **Then** the plugin route is selected
4. **Given** routing decisions at debug log level, **When** signals are routed, **Then** routing decisions are logged showing which tier and route was selected

---

### User Story 7 - Streaming Events with Partial and Final Results (Priority: P3)

Consumers of agent output can distinguish between partial streaming updates (e.g., incremental text generation) and final complete results. A `turn_complete` signal clearly marks when an agent invocation has finished.

**Why this priority**: Enables building responsive UIs that show incremental progress while reliably detecting completion for follow-up actions.

**Independent Test**: Can be fully tested by running an agent that streams output, collecting all events, and verifying partial events are followed by a final event with `is_final_response()` returning true and a `turn_complete` signal.

**Acceptance Scenarios**:

1. **Given** an agent streaming text output, **When** consuming events, **Then** partial events are emitted during generation and a final event with `is_final_response() == true` marks completion
2. **Given** an agent completing a turn, **When** the turn finishes, **Then** a `turn_complete` signal is emitted
3. **Given** a consumer tracking agent progress, **When** partial events arrive, **Then** the UI can update incrementally without triggering completion logic until the final event arrives

---

### User Story 8 - Git Version Control Operations (Priority: P2)

Developers can write agents that perform version control operations (status, diff, log, commit, push, branch management) through a uniform Git backend interface. Agents can inspect repository state, create commits, and push changes while maintaining proper git semantics.

**Why this priority**: Essential for code-focused agent applications that need to manage version control. Git is as fundamental as file operations for code work.

**Independent Test**: Can be fully tested by creating an agent that performs git operations against a test repository, verifying git commands execute correctly and repository state changes as expected.

**Acceptance Scenarios**:

1. **Given** an agent with a Git backend, **When** the agent queries repository status, **Then** the backend returns modified, staged, and untracked files
2. **Given** an agent with Git backend, **When** the agent requests diff output, **Then** the backend returns unified diff format showing changes between revisions
3. **Given** an agent with Git backend, **When** the agent creates a commit with message and author, **Then** the commit is created in the repository with correct metadata
4. **Given** an agent with Git backend, **When** the agent pushes changes to a remote, **Then** changes are transmitted to the remote repository
5. **Given** an agent with Git backend scoped to a specific repository path, **When** git operations are attempted outside the scoped path, **Then** the backend rejects operations with a scope violation error

---

### User Story 9 - HTTP Web Operations (Priority: P2)

Developers can write agents that perform HTTP requests (GET, POST, custom methods) to retrieve web content or interact with REST APIs through a uniform HTTP backend interface. Agents can fetch external data and integrate with web services.

**Why this priority**: Essential for agents that need to access external resources, APIs, and web content. Enables integration with external services beyond local file operations.

**Independent Test**: Can be fully tested by creating an agent that performs HTTP requests against a test server, verifying requests are sent correctly and responses are received and parsed.

**Acceptance Scenarios**:

1. **Given** an agent with an HTTP backend, **When** the agent performs a GET request to a URL, **Then** the backend returns response status, headers, and body content
2. **Given** an agent with HTTP backend, **When** the agent performs a POST request with JSON body, **Then** the backend sends the request with correct content-type and returns the response
3. **Given** an agent with HTTP backend, **When** the agent requests a URL with custom headers, **Then** the backend includes the headers in the request
4. **Given** an agent with HTTP backend, **When** a request exceeds timeout threshold, **Then** the backend returns a timeout error
5. **Given** an agent with HTTP backend, **When** a request fails with network error, **Then** the backend returns a descriptive error with failure reason

---

### User Story 10 - Approval Gates for Risky Operations (Priority: P2)

Developers can configure backends to require user approval before executing potentially destructive or risky operations (file deletion, shell execution, git push, HTTP POST). Approval requests provide context about the operation and wait for user confirmation.

**Why this priority**: Critical for safety when agents perform operations that could cause data loss or unwanted side effects. Enables human-in-the-loop control.

**Independent Test**: Can be fully tested by creating an agent configured with approval gates, attempting a destructive operation, verifying approval is requested with operation details, and confirming operation only executes after approval.

**Acceptance Scenarios**:

1. **Given** an agent with approval-gated backend, **When** the agent attempts a file deletion, **Then** the backend requests approval with file path and operation type before executing
2. **Given** an agent with approval gates, **When** the user approves a pending operation, **Then** the backend executes the operation and returns the result
3. **Given** an agent with approval gates, **When** the user denies a pending operation, **Then** the backend cancels execution and returns a user-denied error
4. **Given** an agent with selective approval gates, **When** the agent performs read-only operations, **Then** operations execute immediately without approval requests
5. **Given** an agent with approval gates, **When** multiple operations require approval, **Then** approvals are requested in execution order with clear operation context

---

### User Story 11 - Enhanced Search with Context and Filtering (Priority: P1)

Developers can write agents that perform advanced code and content searches with ripgrep-style features including context lines, case-insensitive matching, regex patterns, file type filtering, inverted matching, and match counting. Search results include line numbers and surrounding context for better understanding.

**Why this priority**: Essential for code agents that need to find and understand code patterns. Context lines are critical for understanding search results. File type filtering prevents irrelevant matches.

**Independent Test**: Can be fully tested by creating an agent that performs searches with various grep options against test files, verifying context lines are included, line numbers are accurate, and filtering works correctly.

**Acceptance Scenarios**:

1. **Given** an agent with grep backend, **When** searching with context lines (-C 3), **Then** results include 3 lines before and after each match
2. **Given** an agent with grep backend, **When** searching case-insensitively (-i), **Then** matches are found regardless of case
3. **Given** an agent with grep backend, **When** searching with file type filter (--type=rust), **Then** only Rust files are searched
4. **Given** an agent with grep backend, **When** using inverted match (-v), **Then** results show lines NOT matching the pattern
5. **Given** an agent with grep backend, **When** using count mode (-c), **Then** results show match count per file instead of match content
6. **Given** an agent with grep backend, **When** using max-count limit, **Then** search stops after finding specified number of matches

---

### User Story 12 - Process Management and Job Control (Priority: P2)

Developers can write agents that list running processes, terminate processes, spawn background jobs, and manage job control. Agents can monitor long-running operations and clean up processes when needed.

**Why this priority**: Essential for agents that run builds, tests, or long-running operations. Enables proper process lifecycle management and cleanup.

**Independent Test**: Can be fully tested by creating an agent that spawns background processes, lists them, controls them (foreground/background), and terminates them, verifying all operations succeed.

**Acceptance Scenarios**:

1. **Given** an agent with process backend, **When** listing processes, **Then** results include PID, command, CPU usage, and memory usage
2. **Given** an agent with process backend, **When** killing a process by PID, **Then** process is terminated and termination is confirmed
3. **Given** an agent with process backend, **When** spawning a background job, **Then** job runs asynchronously and job ID is returned
4. **Given** an agent with process backend, **When** listing background jobs, **Then** all jobs are shown with status (running/stopped/completed)
5. **Given** an agent with process backend, **When** bringing a background job to foreground, **Then** job output streams to current session

---

### User Story 13 - Archive and Compression Operations (Priority: P2)

Developers can write agents that create, extract, and inspect compressed archives (tar, gzip, zip) for working with packaged code, dependencies, and artifacts. Agents can handle standard archive formats transparently.

**Why this priority**: Essential for agents working with package managers, build artifacts, and compressed downloads. Most software dependencies are distributed as archives.

**Independent Test**: Can be fully tested by creating an agent that creates archives from files, extracts archives to directories, and lists archive contents, verifying all operations preserve file structure and metadata.

**Acceptance Scenarios**:

1. **Given** an agent with archive backend, **When** creating a tar.gz archive from files, **Then** archive is created with correct compression and all files included
2. **Given** an agent with archive backend, **When** extracting a zip archive, **Then** files are extracted to destination preserving directory structure
3. **Given** an agent with archive backend, **When** listing archive contents, **Then** file names, sizes, and permissions are shown without extracting
4. **Given** an agent with archive backend, **When** extracting to a non-empty directory, **Then** backend prompts for conflict resolution or uses configured policy
5. **Given** an agent with archive backend, **When** creating archive with compression level, **Then** compression level is applied correctly

---

### User Story 14 - Working Directory State and Navigation (Priority: P1)

Developers can write agents that maintain persistent working directory state across operations. Directory changes (cd) affect subsequent file operations, enabling natural navigation patterns. Agents can query current directory (pwd) at any time.

**Why this priority**: Critical for natural file navigation. Without persistent state, agents cannot navigate directories logically. Essential for multi-step operations in different directories.

**Independent Test**: Can be fully tested by creating an agent that changes directories multiple times, performs file operations in each, and verifies operations are relative to current directory.

**Acceptance Scenarios**:

1. **Given** an agent with stateful backend, **When** changing directory with cd, **Then** current directory state persists across subsequent operations
2. **Given** an agent with stateful backend, **When** querying current directory with pwd, **Then** correct absolute path is returned
3. **Given** an agent with stateful backend in directory /foo, **When** performing file operation on relative path bar.txt, **Then** operation targets /foo/bar.txt
4. **Given** an agent with stateful backend, **When** changing to non-existent directory, **Then** cd fails and current directory remains unchanged
5. **Given** an agent with stateful backend, **When** cd to .., **Then** current directory moves to parent directory

---

### User Story 15 - Stream Handling and Command Pipelines (Priority: P2)

Developers can write agents that execute command pipelines (cmd1 | cmd2), redirect streams (stdin, stdout, stderr), and compose complex operations from simple commands. Agents can chain operations efficiently.

**Why this priority**: Enables composing complex operations from simple primitives. Essential for data transformation workflows and standard Unix-style command composition.

**Independent Test**: Can be fully tested by creating an agent that executes pipelines with multiple stages, verifies output is correctly piped between commands, and confirms redirects work as expected.

**Acceptance Scenarios**:

1. **Given** an agent with pipeline backend, **When** executing "grep pattern | sort", **Then** grep output is piped to sort and final sorted results returned
2. **Given** an agent with pipeline backend, **When** redirecting stdout to file, **Then** command output is written to file instead of returned
3. **Given** an agent with pipeline backend, **When** redirecting stderr to stdout (2>&1), **Then** both streams are combined in output
4. **Given** an agent with pipeline backend, **When** providing stdin via string, **Then** command reads from provided input instead of prompting
5. **Given** an agent with pipeline backend, **When** pipeline stage fails, **Then** pipeline stops and error from failing stage is reported

---

### User Story 16 - Virtual Filesystem Abstraction (Priority: P1) [IMPLEMENTED]

The `backends` module has been refactored into a Virtual Filesystem (VFS) abstraction providing a filesystem-like interface over heterogeneous data sources. Agents interact with any data source using familiar operations (ls, read, write, cd, cp, mv, etc.) through the `Vfs` trait. Providers declare capabilities via `VfsCapabilities` bitflags. The VFS includes stale-read detection (watch/check_stale) and a ReadGuard preventing blind edits to files that haven't been read. Sandbox concerns (Shell, ProcessManager, ArchiveManager, approval gates) are separated into a dedicated `sandbox` module.

**Why this priority**: Core abstraction that all agent file operations depend on. The Linux coreutils-style interface provides a familiar, composable API surface.

**Independent Test**: VFS conformance suite tests all operations against `MemoryProvider`, `LocalProvider`, and `CompositeProvider`.

**Acceptance Scenarios**:

1. **Given** a VFS provider, **When** capabilities are queried, **Then** the provider accurately reports which operations it supports via `VfsCapabilities` bitflags
2. **Given** a file that has not been read, **When** an edit is attempted, **Then** the ReadGuard rejects the edit with a "must read before edit" error
3. **Given** a file that has been read and subsequently modified externally, **When** an edit is attempted, **Then** the stale-read check detects the change and rejects the edit with `VfsError::StaleRead`
4. **Given** a `CompositeProvider` with multiple mounts, **When** operations target different mount paths, **Then** each operation is routed to the correct underlying provider

---

### User Story 17 - AST-Aware Code Chunking (Priority: P1) [IMPLEMENTED]

Developers can chunk source code files into semantic units (functions, classes, structs, enums, traits, impls) using tree-sitter AST parsing. The chunker supports 14 languages and falls back to recursive text splitting for non-code content. Each chunk is a `Document` with metadata including file path, language, line range, and symbol name.

**Why this priority**: Foundation for semantic search quality. Without AST-aware chunking, embedding-based search treats code as flat text, losing structural boundaries.

**Independent Test**: Chunk a multi-function Rust file and verify each function produces a separate `Document` with correct symbol name and line range metadata.

**Acceptance Scenarios**:

1. **Given** a Rust source file with functions and structs, **When** chunked, **Then** each top-level definition produces a separate `Document` with correct `symbol`, `line_start`, `line_end`, and `language` metadata
2. **Given** a file in an unsupported format, **When** chunked, **Then** the chunker falls back to recursive text splitting without errors
3. **Given** a file with no recognisable definitions, **When** chunked via AST, **Then** an empty result is returned and the caller falls back to text splitting

---

### User Story 18 - Local Embedding and Reranking (Priority: P1) [IMPLEMENTED]

Developers can generate embeddings and perform cross-encoder reranking locally using fastembed-rs models (BAAI/bge family). No external API calls required. The `Embeddings` and `Reranker` traits from synwire-core are implemented for local models.

**Why this priority**: Enables fully offline semantic search. No API keys or network access needed for development/testing.

**Independent Test**: Embed two semantically similar code snippets, verify cosine similarity is high. Embed a dissimilar snippet, verify lower similarity.

**Acceptance Scenarios**:

1. **Given** code snippets with similar semantics, **When** embedded and compared, **Then** cosine similarity exceeds a configurable threshold
2. **Given** a set of candidate documents and a query, **When** reranked with cross-encoder, **Then** the most relevant document is ranked first

---

### User Story 19 - Vector Store with LanceDB (Priority: P1) [IMPLEMENTED]

Developers can store and query document embeddings using LanceDB as the vector store backend. Implements the `VectorStore` trait with `add_documents` and `similarity_search_with_score`.

**Why this priority**: Persistent, efficient vector storage enabling semantic search across indexed codebases.

**Independent Test**: Add documents to store, query by similarity, verify ranked results include the expected documents.

**Acceptance Scenarios**:

1. **Given** documents added to the vector store, **When** a similarity search is performed, **Then** results are returned ranked by score with the most similar document first
2. **Given** an empty store, **When** a search is performed, **Then** an empty result set is returned without errors

---

### User Story 20 - Semantic Indexing Pipeline (Priority: P1) [IMPLEMENTED]

Developers can index entire directory trees for semantic search. The pipeline walks directories, chunks files using AST-aware chunking, generates embeddings, and stores vectors in LanceDB. Indexing runs asynchronously with progress events, supports incremental re-indexing via content hash tracking (xxh128), and includes a file watcher for auto-reindexing on changes. Results support optional cross-encoder reranking.

**Why this priority**: End-to-end semantic search capability. Agents can find code by concept ("authentication logic", "error handling") rather than just exact text matches.

**Independent Test**: Index a test directory, perform semantic search for a concept, verify relevant files are returned with correct metadata.

**Acceptance Scenarios**:

1. **Given** a directory tree, **When** indexed, **Then** all text files are chunked, embedded, and stored with progress events emitted
2. **Given** an already-indexed directory with one changed file, **When** re-indexed without force, **Then** only the changed file is re-processed (verified via content hash)
3. **Given** an indexed directory, **When** a semantic search is performed, **Then** results include file path, line range, content, similarity score, and optional symbol name
4. **Given** an index in progress, **When** a search is attempted, **Then** `VfsError::IndexNotReady` is returned

---

### User Story 21 - LSP Client Integration (Priority: P2) [IMPLEMENTED]

Agents can interact with Language Server Protocol servers to get structural code intelligence: go-to-definition, find-references, hover information, workspace symbols, diagnostics, code actions, formatting, and rename. Tools are generated conditionally based on the server's advertised capabilities. A registry manages multiple language servers.

**Why this priority**: Provides precise, compiler-grade code navigation. Complements semantic search with exact structural queries (what calls this function? where is this type defined?).

**Independent Test**: Start an LSP server, query workspace symbols, verify results match expected definitions.

**Acceptance Scenarios**:

1. **Given** a running LSP server with definition support, **When** `lsp_goto_definition` is called at a symbol position, **Then** the definition location is returned
2. **Given** an LSP server without rename support, **When** tools are generated, **Then** `lsp_rename` is not included in the tool set
3. **Given** multiple language servers registered, **When** a file is opened, **Then** the correct server is selected based on language

---

### User Story 22 - Debug Adapter Protocol Integration (Priority: P2) [IMPLEMENTED]

Agents can debug programs through the Debug Adapter Protocol — setting breakpoints, stepping through code, evaluating expressions, inspecting variables, and viewing disassembly. A DAP client manages debug sessions with tools for all standard debug operations.

**Why this priority**: Enables agents to debug failing tests and understand runtime behaviour, complementing static analysis.

**Independent Test**: Launch a debug session, set a breakpoint, run to breakpoint, evaluate an expression, verify the result.

**Acceptance Scenarios**:

1. **Given** a debug session with a breakpoint set, **When** the program runs, **Then** execution pauses at the breakpoint and context is available
2. **Given** a paused debug session, **When** `evaluate` is called with an expression, **Then** the result is returned
3. **Given** a running debug session, **When** `step` is called, **Then** execution advances one step and the new position is reported

---

### User Story 23 - Process Sandboxing (Priority: P2) [IMPLEMENTED]

Agents execute in sandboxed environments with process isolation, output capture, and visibility controls. The sandbox provides a process registry, platform-specific isolation (Linux namespaces), and a plugin interface for lifecycle management.

**Why this priority**: Safety-critical for production agent deployment. Prevents agents from affecting the host system beyond their allowed scope.

**Independent Test**: Spawn a sandboxed process, verify it cannot access files outside its scope, verify output is captured.

**Acceptance Scenarios**:

1. **Given** a sandboxed process, **When** it attempts to access files outside its scope, **Then** the access is denied
2. **Given** a sandboxed process producing output, **When** the output is queried, **Then** stdout and stderr are captured and available
3. **Given** sandbox visibility controls, **When** configured to hide certain operations, **Then** those operations are not visible to the agent

---

### User Story 24 - Per-Method AST Chunking (Priority: P1) [DRAFT]

Developers get fine-grained semantic search results at the individual method level rather than entire `impl` blocks or class bodies. The chunker recurses one level into container nodes (`impl_item`, `class_body`, `class_declaration`) to produce per-method chunks with the parent type as context prefix in metadata. This dramatically improves search precision for languages with impl blocks / class bodies.

**Why this priority**: Research shows localization quality dominates end-to-end agent performance. Current top-level-only chunking produces oversized chunks for Rust `impl` blocks, making it impossible to distinguish between methods within the same type. This is the smallest change with the largest search quality improvement.

**Independent Test**: Chunk a Rust file with an `impl` block containing 5 methods, verify 5 separate documents are produced, each with correct symbol name and parent type in metadata.

**Acceptance Scenarios**:

1. **Given** a Rust `impl Foo` block with methods `bar()`, `baz()`, and `qux()`, **When** chunked, **Then** three separate `Document`s are produced with symbols `Foo::bar`, `Foo::baz`, `Foo::qux`
2. **Given** a Python class with multiple methods, **When** chunked, **Then** each method produces a separate `Document` with the class name as context prefix
3. **Given** a top-level function (not inside a container), **When** chunked, **Then** behaviour is unchanged from current chunking

---

### User Story 25 - File Skeleton / API Summary Generation (Priority: P1) [DRAFT]

Agents can request a compact structural summary of any source file — class headers, method signatures, type definitions — without reading full file contents. The `skeleton` VFS tool uses tree-sitter to extract definition nodes and emit only their signatures (first line / declaration), producing a token-efficient overview suitable for LLM localization.

**Why this priority**: Research (Agentless, Xia et al.) shows that hierarchical narrowing using file skeletons is the most cost-efficient localization approach ($0.34/issue). Reading full files wastes context window budget; skeletons provide the same localization signal at a fraction of the token cost.

**Independent Test**: Generate skeleton for a file with 10 functions, verify output contains all 10 signatures but no function bodies, and is <20% of full file token count.

**Acceptance Scenarios**:

1. **Given** a Rust source file with functions, structs, and impls, **When** `skeleton` is called, **Then** output contains function signatures, struct definitions, and impl headers without bodies
2. **Given** a file in an unsupported language, **When** `skeleton` is called, **Then** the full file is returned (graceful fallback)
3. **Given** a 500-line source file, **When** `skeleton` is called, **Then** the output is significantly smaller than the full file content

---

### User Story 26 - Hierarchical Narrowing Localization (Priority: P1) [DRAFT]

Agents can perform efficient code localization using a three-phase narrowing pipeline: (1) directory tree → LLM ranks suspicious files, (2) file skeletons or document symbols → LLM ranks functions, (3) read specific line ranges of top candidates. This Agentless-style approach matches complex agent systems at a fraction of the cost.

**Why this priority**: Research consistently shows localization is the bottleneck, not code generation. Hierarchical narrowing achieves 27.3%+ resolve rates on SWE-bench Lite at $0.34/issue. This can be implemented as a middleware or compound tool composing existing VFS operations.

**Independent Test**: Given a bug description and a test codebase, run the narrowing pipeline and verify the correct file and function are identified within the top-3 candidates.

**Acceptance Scenarios**:

1. **Given** an issue description and a repository, **When** the narrowing pipeline runs phase 1, **Then** a ranked list of suspicious files is produced using directory tree context
2. **Given** the top-ranked files from phase 1, **When** phase 2 runs with file skeletons, **Then** a ranked list of suspicious functions/methods is produced
3. **Given** the top-ranked functions from phase 2, **When** phase 3 reads the relevant line ranges, **Then** the agent has precise context for generating a fix

---

### User Story 27 - Code Dependency Graph Construction (Priority: P2) [DRAFT]

Developers can build and query a code dependency graph capturing definition→reference edges, import relationships, and call edges across files. The graph is constructed from tree-sitter ASTs and stored alongside the vector index. Agents can traverse the graph to follow call chains, find all callers/callees, and understand cross-file dependencies.

**Why this priority**: Research shows 69.7% of successfully localised bugs require multi-hop graph traversals (KGCompass). Flat embedding retrieval fundamentally cannot solve bugs that span multiple files connected by call chains. The graph enables "what calls this function?" and "what does this function depend on?" queries.

**Independent Test**: Build graph for a multi-file project, query callers of a function, verify all call sites are returned including transitive callers at depth 2.

**Acceptance Scenarios**:

1. **Given** a Rust project with cross-file function calls, **When** the code graph is built, **Then** definition→reference edges correctly link callers to callees across files
2. **Given** a symbol in the graph, **When** `graph_query(symbol, depth=2, direction=callers)` is called, **Then** both direct and transitive callers are returned
3. **Given** a semantic search query, **When** `graph_search(query, hops=2)` is called, **Then** the nearest embedding match is found and its ego-graph is expanded, returning the subgraph of related code

---

### User Story 28 - Hybrid BM25 + Vector Search (Priority: P2) [DRAFT]

Agents can search code using both lexical (BM25/TF-IDF) and semantic (vector embedding) approaches simultaneously. BM25 catches exact identifier matches that embeddings miss; embeddings catch semantic paraphrases that BM25 misses. Results are combined with configurable alpha weighting.

**Why this priority**: Research consistently shows hybrid retrieval outperforms either approach alone. An agent searching for `validate_token` needs BM25 for the exact match, but searching for "authentication logic" needs embeddings. Hybrid search handles both.

**Independent Test**: Search for an exact function name — verify BM25 component finds it even if embedding similarity is low. Search for a conceptual description — verify embedding component finds relevant code even without keyword matches.

**Acceptance Scenarios**:

1. **Given** an indexed codebase, **When** `hybrid_search("validate_token", alpha=0.5)` is called, **Then** results include both exact identifier matches (BM25) and semantically similar code (vector)
2. **Given** alpha=1.0 (pure BM25), **When** searching, **Then** results are identical to a lexical search
3. **Given** alpha=0.0 (pure vector), **When** searching, **Then** results are identical to current semantic search

---

### User Story 29 - Test-Guided Fault Localization (Priority: P2) [DRAFT]

When tests fail, agents can use code coverage data from DAP to compute spectrum-based fault localization (SBFL) scores, then combine these with semantic search and LLM reranking for highly accurate fault localization. The Ochiai scoring algorithm ranks functions by suspiciousness based on which tests pass/fail through them.

**Why this priority**: Research shows SBFL+LLM reranking achieves 68.4% improvement in Top-1 localization accuracy (FuseFL). This integrates the existing DAP crate with the semantic search pipeline for a powerful debugging workflow.

**Independent Test**: Given a test suite where one test fails, compute Ochiai scores, verify the buggy function has the highest suspiciousness score.

**Acceptance Scenarios**:

1. **Given** a failing test with coverage data from DAP, **When** SBFL scores are computed, **Then** functions executed by the failing test but not by passing tests receive the highest Ochiai scores
2. **Given** SBFL-ranked functions and a bug description, **When** LLM reranking is applied, **Then** the true buggy function is promoted to the top rank more often than with either approach alone
3. **Given** no coverage data available, **When** test-guided localization is requested, **Then** the system gracefully falls back to standard semantic search

---

### User Story 30 - Repository Memory and Experience Pool (Priority: P2) [DRAFT]

Agents maintain a persistent memory of past edits, issue→file associations, and LLM-generated file summaries across sessions. When investigating a new issue, the agent can query "what files were touched when we last fixed an auth bug?" and receive answers from memory rather than re-searching the entire codebase.

**Why this priority**: Research (EvoCoder) shows that cross-session experience pools achieve 20% improvement over stateless approaches. Builds on existing session/checkpoint infrastructure to create a repository-scoped knowledge base.

**Independent Test**: Record an edit association in session A, query it in session B, verify the association is returned.

**Acceptance Scenarios**:

1. **Given** an agent that edited files X, Y, Z to fix issue "auth timeout", **When** the edit is recorded in the experience pool, **Then** subsequent sessions can query "auth" and find files X, Y, Z
2. **Given** a file that has been edited frequently, **When** its summary is queried, **Then** the most recent LLM-generated summary is returned
3. **Given** the experience pool, **When** queried for files related to a concept, **Then** results include both past edit associations and file summaries

---

### User Story 31 - Dynamic Call Graph Construction (Priority: P3) [DRAFT]

Agents build code dependency graphs incrementally during search by following edges on demand via LSP go-to-definition, rather than pre-computing a full static graph. This avoids expensive upfront computation for large repositories and adapts the graph to the specific search task.

**Why this priority**: Research (CoSIL) shows dynamic graph construction achieves comparable accuracy to static graphs without the upfront cost. For repositories with millions of lines of code, pre-computing a full graph is impractical. This composites LSP navigation with semantic search.

**Independent Test**: Starting from a function identified by semantic search, follow call edges via LSP for 3 hops, verify the resulting subgraph matches the static graph for those nodes.

**Acceptance Scenarios**:

1. **Given** a starting symbol from semantic search, **When** the agent follows a definition edge via LSP, **Then** the edge is added to the working graph and the target symbol's context is available
2. **Given** a dynamically-built subgraph, **When** the agent reaches a symbol already in the graph, **Then** a cycle is detected and traversal stops on that path
3. **Given** no LSP server available, **When** dynamic graph construction is attempted, **Then** the system falls back to static graph or grep-based navigation

---

### User Story 32 - MCTS-Based Search Trajectories (Priority: P3) [DRAFT]

Agents explore multiple localization and repair trajectories in parallel using Monte Carlo Tree Search. A value function scores each exploration path, and the agent allocates more compute to promising paths. This enables inference-time scaling — more compute budget yields better results.

**Why this priority**: Research (SWE-Search) shows MCTS achieves 23% relative improvement across models, and performance scales consistently with search depth. This is the primary axis for trading compute for accuracy.

**Independent Test**: Given a localization task, run MCTS with depth 3 vs depth 1, verify depth 3 identifies the correct location more often.

**Acceptance Scenarios**:

1. **Given** a localization task, **When** MCTS explores multiple paths, **Then** the path with the highest value function score identifies the correct code location
2. **Given** a configurable compute budget, **When** the budget is increased, **Then** MCTS explores more trajectories and accuracy improves
3. **Given** a single-path baseline, **When** compared to MCTS, **Then** MCTS resolves at least 15% more issues

---

### User Story 33 - Self-Evolving Tool Creation and Agent Skills (Priority: P3) [DRAFT]

Agents can create ad-hoc tools during a session based on reflection on past steps, using multiple execution backends: embedded scripting (Lua via `mlua` or Rhai), WebAssembly plugins (via Extism), or sequences of existing tool invocations. Tools can be packaged as portable, shareable **agent skills** following the [Agent Skills specification](https://agentskills.io/specification) — a skill is a directory containing a `SKILL.md` file with YAML frontmatter (name, description) and Markdown instructions, plus optional `scripts/`, `references/`, and `assets/` directories. A new `synwire-agent-skills` crate provides skill discovery, validation, loading, and execution with support for embedded runtimes (Lua, Rhai, Extism/WASM) in addition to the standard script execution model. Skills are distributable, discoverable, and loadable by any synwire-based agent or MCP server.

**Why this priority**: Research (Live-SWE-agent) achieved 77.4% on SWE-bench Verified with self-evolving tools. Adding embedded scripting runtimes (Lua/Rhai for lightweight, Extism/WASM for portable sandboxed plugins) makes tool creation practical without requiring Rust compilation. The agent-skills crate standardises the format so tools are reusable across sessions, projects, and products — an industry-standard skill ecosystem rather than throwaway session-scoped scripts.

**Independent Test**: Agent creates a Lua tool that wraps a 3-step grep+filter+format pattern, verifies it executes correctly. Package it as an agent-skill manifest, load it in a fresh session, verify it works identically. Load an Extism WASM skill, verify it runs sandboxed with no host filesystem access.

**Acceptance Scenarios**:

1. **Given** an agent that has performed the same multi-step pattern twice, **When** it emits a `CreateTool` directive with a Lua/Rhai script implementation, **Then** the new tool is registered and available for subsequent turns
2. **Given** a dynamically-created tool, **When** the session ends, **Then** the tool definition is persisted as an agent-skill manifest in the experience pool for future sessions
3. **Given** a dynamically-created tool, **When** the tool attempts a restricted operation, **Then** sandbox and permission checks apply identically to native tools
4. **Given** a skill directory containing a `SKILL.md` with valid frontmatter, **When** loaded by the skill loader, **Then** the skill is registered with its name and description available for discovery, and full instructions loaded on activation
5. **Given** a Lua skill and a Rhai skill implementing the same logic in `scripts/`, **When** both are loaded, **Then** both produce identical tool outputs for identical inputs
6. **Given** an Extism WASM skill, **When** executed, **Then** it runs in a sandboxed WASM runtime with only capabilities declared in `allowed-tools` granted
7. **Given** skills in `$DATA/<product>/skills/` and `.<product>/skills/` at the project root, **When** the MCP server starts, **Then** all valid skills are auto-discovered via progressive disclosure (name + description only at startup)
8. **Given** a `SKILL.md` with `name: pdf-processing` in a directory named `pdf-processing/`, **When** validated, **Then** validation passes. **Given** the directory is named `other-name/`, **Then** validation fails with a name mismatch error

---

### User Story 34 - Dataflow-Guided Retrieval (Priority: P3) [DRAFT]

Agents can follow data dependencies — tracking where a variable's value originates and where it flows — to locate relevant code across files. Unlike control-flow-based navigation, dataflow retrieval answers "where does this value come from?" and "what consumes this result?".

**Why this priority**: Many bugs involve incorrect data transformations across multiple functions. Control flow navigation (call graphs) misses cases where data flows through return values, struct fields, or channel sends/receives. Dataflow analysis complements structural navigation.

**Independent Test**: Given a variable at a specific location, trace its data source across two function boundaries, verify the origin is correctly identified.

**Acceptance Scenarios**:

1. **Given** a variable at a specific code location, **When** dataflow retrieval traces its source, **Then** the originating assignment or parameter is identified even across function boundaries
2. **Given** a function return value, **When** dataflow retrieval traces its consumers, **Then** all call sites that use the return value are identified
3. **Given** a language without LSP dataflow support, **When** dataflow retrieval is attempted, **Then** the system falls back to tree-sitter-based heuristic analysis or grep

---

### User Story 35 - GraphRAG Community Detection for Code (Priority: P2) [DRAFT]

Agents can discover hierarchical community structure over the code dependency graph using HIT-Leiden community detection. Code entities (functions, types, modules) are grouped into communities of related concepts at multiple resolution levels. Each community receives an LLM-generated summary. Agents search at the community level first (coarse — "networking cluster"), then drill into specific members (fine — individual HTTP handler functions). When files change, community structure is updated incrementally (63-136x faster than full reclustering) and only affected community summaries are regenerated.

**Why this priority**: This is the GraphRAG architecture applied to code — hierarchical community structure enables multi-resolution search that neither flat embeddings nor raw graph traversal can achieve. The incremental update capability (via `CommunityState::update()`) makes it practical for live development where files change constantly. Builds directly on US27 (code dependency graph) and integrates with the existing file watcher in `synwire-index`.

**Independent Test**: Build a code graph for a multi-module project, run community detection, verify that modules with high internal coupling form distinct communities. Change one file, run incremental update, verify only affected communities change and the update is at least 10x faster than full reclustering.

**Acceptance Scenarios**:

1. **Given** a code dependency graph from US27, **When** HIT-Leiden community detection runs, **Then** a hierarchical community partition is produced where strongly-connected code clusters form distinct communities at each level
2. **Given** an established community structure, **When** a file changes and delta edges are computed, **Then** `CommunityState::update()` incrementally updates only affected communities without full reclustering
3. **Given** communities at multiple hierarchy levels, **When** `community_search("authentication")` is called, **Then** the agent finds the auth community via summary search, then drills into its member functions/types
4. **Given** a community with 20 member functions, **When** its summary is requested, **Then** an LLM-generated summary describes the community's purpose and key members, usable as compressed context
5. **Given** `CommunityState` serialised via `into_parts()`, **When** the state is restored via `from_parts()`, **Then** incremental updates continue correctly without full reclustering

---

### User Story 36 - Configurable Persistent Storage Layout (Priority: P1) [DRAFT]

Products built on synwire (coding agents, CLI tools, IDE extensions) can configure where all persistent data lives — sessions, indices, graphs, experience pools, community state — through a single `StorageLayout` struct parameterised by product name. Data is split between durable storage (expensive to recreate: session checkpoints, community summaries, experience pool) and cache storage (rebuildable: vector indices, BM25 indices, LSP caches, downloaded models). Projects are identified by a stable `ProjectId` (first Git commit hash) that survives directory moves and works across developer machines. The existing checkpoint crates (`synwire-checkpoint`, `synwire-checkpoint-sqlite`) are unmodified — they already accept path parameters — and `StorageLayout` becomes the coordination layer that computes paths for each subsystem.

**Why this priority**: Without this, every subsystem invents its own storage location (index hardcodes `$CACHE/synwire/indices/`, sessions use ad-hoc paths). Multiple products built on synwire would collide under the same `synwire/` directory. Project data would be orphaned when directories move. And developers couldn't run two different agents on the same project without corruption. This is infrastructure that all other persistence depends on.

**Independent Test**: Create two `StorageLayout` instances with different product names, verify their paths are fully isolated. Create a `ProjectId` from a Git repository, move the repository to a new path, verify the same `ProjectId` is computed. Verify that `SqliteSaver` and `SemanticIndex` both receive correct paths from the layout without any hardcoded assumptions.

**Acceptance Scenarios**:

1. **Given** two products ("claude-code" and "acme-agent") built on synwire, **When** both are used on the same machine, **Then** their persistent data is fully isolated under separate product directories with zero path overlap
2. **Given** a Git repository cloned at `/home/alice/projects/foo`, **When** `ProjectId` is computed, **Then** it uses the first commit hash. **When** the repo is moved to `/home/alice/work/foo`, **Then** `ProjectId` remains the same and cached data is found
3. **Given** two developers cloning the same repository to different paths, **When** `ProjectId` is computed on each machine, **Then** both get the same ID (enabling shared CI cache strategies)
4. **Given** a non-Git directory, **When** `ProjectId` is computed, **Then** it falls back to `sha256(canonical_path)` maintaining current behaviour
5. **Given** `StorageLayout` configured with a root override, **When** paths are requested, **Then** all storage (durable + cache) is rooted under the override path (for CI, Docker, portable installs)
6. **Given** a `StorageLayout`, **When** `session_db(session_id)` is called, **Then** the returned path is passed to `SqliteSaver::new()` unchanged — the checkpoint crate is unaware of the layout
7. **Given** a `StorageLayout`, **When** `index_cache(project_id)` is called, **Then** the returned path is passed to `IndexConfig.cache_base` — the index crate is unaware of the layout
8. **Given** two agent instances targeting the same project concurrently, **When** both attempt to write to the same data, **Then** advisory file locking (`flock`) prevents corruption
9. **Given** a subsystem whose storage schema changes between versions, **When** the agent starts, **Then** the migration framework detects the version mismatch and runs the appropriate migration before the subsystem accesses data

---

### User Story 37 - Automatic Repository Clone and Mount (Priority: P1) [DRAFT]

Agents can clone a Git repository by URL and immediately mount it into the VFS as a searchable, browsable `LocalProvider`. The cloned repository is automatically available to all existing VFS tools (read, grep, tree, glob, find, semantic_search) and optionally indexed for semantic search. A middleware detects when the agent is repeatedly fetching individual files from the same GitHub repository via HTTP and suggests cloning the entire repository instead.

**Why this priority**: This is a common workflow friction point — agents waste tokens and time fetching files one-by-one from GitHub when having the full repository locally would enable grep, tree, semantic search, and cross-file navigation. Automating the clone-mount-index cycle removes this friction entirely. The detection middleware prevents the wasteful pattern before the user has to intervene.

**Independent Test**: Call `clone_repo` with a public GitHub URL, verify the repository is cloned to the cache directory, mounted into the VFS, and immediately searchable via `grep` and `tree`. Trigger the detection middleware by making 3+ web fetches from the same repository, verify a clone suggestion is emitted.

**Acceptance Scenarios**:

1. **Given** a public GitHub repository URL, **When** `clone_repo(url)` is called, **Then** the repository is cloned to the storage layout's repo cache directory and mounted into the `CompositeProvider` at a path like `/repos/<owner>/<repo>/`
2. **Given** a cloned and mounted repository, **When** `grep("pattern", path="/repos/owner/repo/")` is called, **Then** search results are returned identically to any other VFS path
3. **Given** a cloned repository with `index: true` option, **When** the clone completes, **Then** semantic indexing is automatically triggered and `semantic_search` becomes available once indexing finishes
4. **Given** an agent that has made 3+ `web_fetch` calls to `raw.githubusercontent.com/owner/repo/*`, **When** the detection middleware recognises the pattern, **Then** a `PromptSuggestion` event is emitted suggesting cloning the repository for faster access
5. **Given** a previously-cloned repository, **When** `clone_repo` is called again for the same URL, **Then** the existing clone is updated via `git pull` rather than re-cloned from scratch
6. **Given** cloned repositories in the cache, **When** `repo_gc(max_age_days)` is called, **Then** repositories not accessed within the configured period are removed to reclaim disk space
7. **Given** a repository URL with a specific branch or tag, **When** `clone_repo(url, ref: "v2.0")` is called, **Then** the clone checks out the specified ref
8. **Given** a private repository requiring authentication, **When** `clone_repo` is called, **Then** it uses the ambient Git credential configuration (SSH keys, credential helpers) — synwire does not manage Git credentials

---

### User Story 38 - Standalone MCP Server Binary (Priority: P0) [DRAFT]

Developers can run synwire's code intelligence tools — semantic search, code graph queries, file skeletons, hierarchical narrowing, hybrid search, repository clone-and-mount, and LSP/DAP integration — as a standalone MCP (Model Context Protocol) server binary. This server is usable immediately with any MCP-compatible client (Claude Code, GitHub Copilot, Cursor, etc.) without building a full agent. The binary exposes VFS tools, search tools, and code navigation tools as MCP tool definitions, backed by the `StorageLayout` persistence layer.

**Why this priority**: P0 because this provides immediate value — the tools we're building can be used *today* in existing AI coding assistants via MCP, rather than waiting for the full agent runtime to be complete. It also serves as a real-world integration test for all the underlying crates (chunker, index, embeddings, vector store, LSP, DAP, VFS, storage layout).

**Independent Test**: Start the MCP server binary, connect Claude Code to it via stdio transport, index a project, perform semantic search, verify results appear in the Claude Code conversation.

**Acceptance Scenarios**:

1. **Given** the MCP server binary installed, **When** configured in Claude Code's MCP settings (`claude_desktop_config.json` or `.claude/settings.json`), **Then** all synwire tools appear as available MCP tools in the conversation
2. **Given** a connected MCP client, **When** the `index` tool is called with a project path, **Then** semantic indexing runs in the background and the client receives progress notifications
3. **Given** an indexed project, **When** `semantic_search`, `grep`, `tree`, `glob`, `skeleton`, or `hybrid_search` tools are called, **Then** results are returned in MCP tool result format
4. **Given** the MCP server with LSP integration enabled, **When** `lsp_goto_definition` or `lsp_find_references` is called, **Then** the server manages the language server lifecycle transparently and returns results
5. **Given** the MCP server, **When** `clone_repo` is called with a GitHub URL, **Then** the repository is cloned, mounted, and immediately searchable via all tools
6. **Given** two editor instances (e.g., Claude Code and Copilot) each spawning their own MCP server process for the same project, **When** both query the index concurrently, **Then** the shared on-disk index is used with proper read locking — no corruption or stale reads
7. **Given** one MCP server instance that completes indexing, **When** a second instance for the same project runs a search, **Then** the second instance sees the updated index without re-indexing
8. **Given** the server binary, **When** started with `--product-name my-agent` flag, **Then** all persistence uses that product name in the `StorageLayout`, enabling isolation between different tool configurations
9. **Given** the editor closing, **When** the stdio pipe is closed, **Then** the MCP server process exits cleanly, cancelling any in-progress background work

---

### User Story 39 - Tool Search and Progressive Tool Discovery (Priority: P0) [DRAFT]

Any agent built on synwire's tool system — whether using the synwire agent runtime, a third-party agent framework like Claude Code, or the MCP server — faces tool count explosion. With 40+ tools across VFS, search, graph, communities, LSP, DAP, and skills, LLM accuracy degrades sharply (49% at 58 tools per Anthropic benchmarks). The synwire-core tool registry MUST support progressive tool discovery as a framework-level capability: tools are registered with namespaces and example queries, a `ToolSearchIndex` provides embedding-based retrieval, and a `tool_search` meta-tool is available for any agent to include. Third-party agents (Claude Code, Cursor, etc.) using synwire tools via MCP get this automatically via the MCP server's deferred loading. Agents using synwire-core directly get it via the `ToolSearchIndex` API.

**Why this priority**: P0 because without this, any agent using synwire's full tool set will suffer accuracy collapse. This is a framework-level correctness requirement, not an MCP-specific feature. Research shows 85% token reduction and +25pp accuracy improvement with tool search.

**Independent Test**: Register 40 tools with a `ToolSearchIndex`, query "find where function X is defined", verify top-3 results include `lsp_goto_definition` and `grep`. Verify initial tool listing (names+descriptions only) consumes <2,000 tokens.

**Acceptance Scenarios**:

1. **Given** 40+ tools registered with a `ToolSearchIndex`, **When** an agent requests the tool listing for LLM context, **Then** it receives only `name` + one-line `description` per tool (~50 tokens each, <2,000 total) — not full JSON Schemas
2. **Given** a natural language query, **When** `tool_search("find where function X is defined")` is called, **Then** the index returns full schemas for the top-K most relevant tools (default K=5) based on embedding similarity
3. **Given** a namespace query, **When** `tool_search("namespace:graph")` is called, **Then** all tools in the `graph` namespace are returned with full schemas
4. **Given** tools grouped into namespaces, **When** the tool listing is generated, **Then** each tool name is prefixed with its namespace (e.g., `file.read`, `search.semantic`, `graph.query`) enabling the LLM to understand organisation
5. **Given** a third-party agent using synwire tools via MCP, **When** the MCP server exposes tools, **Then** tools are automatically deferred (name+description only) and `tool_search` is included as a bootstrap tool
6. **Given** an agent using synwire-core directly, **When** it constructs its tool set, **Then** `ToolSearchIndex` provides the same retrieval API without requiring MCP

---

### User Story 40 - Multi-Server MCP Client (Priority: P1)

Developers can connect to multiple MCP servers simultaneously and aggregate their tools into a single unified tool set. The `MultiServerMcpClient` accepts a named map of server connections (stdio, SSE, StreamableHttp, WebSocket), establishes connections in parallel, and exposes all discovered tools as Synwire `Tool` implementations. When the `tool_name_prefix` flag is set, tool names are prefixed with the server name to avoid collisions.

**Why this priority**: Core entry point for MCP integration — without this, agents cannot use external MCP tools. Multi-server support is essential for real-world deployments where tools are spread across multiple servers.

**Independent Test**: Can be fully tested by starting two mock MCP servers (one stdio, one HTTP), creating a `MultiServerMcpClient` with both, calling `get_tools()`, and verifying tools from both servers are returned with correct prefixing.

**Acceptance Scenarios**:

1. **Given** a `MultiServerMcpClient` configured with two named servers, **When** `connect()` is called, **Then** both servers are connected simultaneously (not sequentially) and connection status is reported per server
2. **Given** a connected multi-server client with `tool_name_prefix: true`, **When** `get_tools()` is called, **Then** each tool name is prefixed as `{server_name}_{tool_name}` with server names sanitised for valid identifiers
3. **Given** a connected multi-server client, **When** one server becomes unhealthy, **Then** tools from the remaining healthy servers continue to be available and the unhealthy server's tools are excluded with a health status error
4. **Given** a multi-server client, **When** tool listing exceeds a single page, **Then** cursor-based pagination fetches all tools with a safeguard capping iteration at 1000 pages
5. **Given** a multi-server client, **When** a tool is invoked by name, **Then** the call is routed to the correct server based on tool name (or prefix)

---

### User Story 41 - MCP-to-Synwire Tool Conversion (Priority: P1)

Developers can convert MCP tool definitions into Synwire `Tool` implementations and vice versa, enabling bidirectional interoperability. MCP tools discovered from servers are automatically wrapped as Synwire tools with correct schema, description, and content type mapping. Synwire tools can be exposed as MCP tools with validated schemas.

**Why this priority**: Without bidirectional conversion, MCP tools cannot be used in Synwire agents and Synwire tools cannot be exposed to MCP clients. This is the core interoperability layer.

**Independent Test**: Can be fully tested by round-tripping a tool definition: create a Synwire tool, convert to MCP, convert back to Synwire, and verify all metadata (name, description, schema, annotations) is preserved.

**Acceptance Scenarios**:

1. **Given** an MCP tool definition, **When** `convert_mcp_tool_to_synwire_tool()` is called, **Then** the resulting Synwire `Tool` has matching name, description, and input schema
2. **Given** a Synwire tool, **When** `to_mcp_tool()` is called, **Then** the resulting MCP tool definition has a valid `args_schema` and any injected arguments are rejected with an error
3. **Given** MCP tool content types (Text, Image, ResourceLink, EmbeddedResource), **When** content is converted, **Then** each type maps correctly to its Synwire representation
4. **Given** MCP AudioContent, **When** conversion is attempted, **Then** the system returns `UnsupportedContent` since Synwire does not yet model audio
5. **Given** an MCP tool result with `isError: true`, **When** converted, **Then** a `ToolException` is raised

---

### User Story 42 - MCP Resource and Prompt Retrieval (Priority: P2)

Developers can load MCP resources as blob equivalents and retrieve MCP prompts as Synwire message types. Resources are fetched excluding dynamic resources, and prompts handle role-based mapping and multi-content support.

**Why this priority**: Resources and prompts are part of the MCP specification and needed for complete MCP support. Resource retrieval enables context loading; prompt retrieval enables template-based workflows.

**Independent Test**: Can be fully tested by creating a mock MCP server with resources and prompts, fetching them via the adapter, and verifying correct conversion to Synwire types.

**Acceptance Scenarios**:

1. **Given** an MCP server with static resources, **When** `get_resources()` is called, **Then** resources are returned as `McpBlob` equivalents with dynamic resources excluded
2. **Given** an MCP server with a prompt template, **When** `get_prompt()` is called with arguments, **Then** the prompt is converted to Synwire `Message` types with correct role mapping
3. **Given** an MCP prompt with multi-content messages, **When** converted, **Then** all content parts are preserved in the Synwire message model

---

### User Story 43 - Tool Call Interceptors (Priority: P2)

Developers can compose multiple interceptors around MCP tool calls in an onion/middleware pattern. Each interceptor can inspect, modify, or short-circuit tool invocations. Interceptors are panic-safe — a failing interceptor does not corrupt the call chain.

**Why this priority**: Enables cross-cutting concerns like logging, rate limiting, caching, and access control around tool calls without modifying tool implementations.

**Independent Test**: Can be fully tested by registering three interceptors (logging, timing, validation) and verifying they execute in correct onion order — outer interceptor sees both request and response, inner interceptor is closest to the tool.

**Acceptance Scenarios**:

1. **Given** three interceptors A, B, C registered in order, **When** a tool is called, **Then** they execute in onion order: A-before → B-before → C-before → tool → C-after → B-after → A-after
2. **Given** an interceptor that short-circuits with a result, **When** a tool is called, **Then** inner interceptors and the tool are skipped and the short-circuit result is returned through remaining outer interceptors
3. **Given** an interceptor that panics, **When** a tool is called, **Then** the panic is caught and converted to an error result without corrupting the interceptor chain

---

### User Story 44 - Tool Provider Abstraction (Priority: P1)

Developers can compose tool sources from multiple providers — static tool sets, MCP-backed tools, and composite aggregations — through a unified `ToolProvider` trait. Agents discover tools at runtime without knowing the tool source.

**Why this priority**: Decouples tool registration from tool source. Agents can use the same API whether tools come from code, MCP servers, or dynamic discovery.

**Independent Test**: Can be fully tested by creating a `CompositeToolProvider` with a `StaticToolProvider` (3 tools) and an `McpToolProvider` (2 tools), calling `discover_tools()`, and verifying all 5 tools are returned.

**Acceptance Scenarios**:

1. **Given** a `StaticToolProvider` with a fixed tool set, **When** `discover_tools()` is called, **Then** exactly the configured tools are returned
2. **Given** an `McpToolProvider` backed by a `MultiServerMcpClient`, **When** `discover_tools()` is called, **Then** tools from all connected MCP servers are returned
3. **Given** a `CompositeToolProvider` aggregating multiple providers, **When** `get_tool(name)` is called, **Then** the correct tool is returned regardless of which sub-provider owns it
4. **Given** a `CompositeToolProvider` with duplicate tool names across providers, **When** tools are aggregated, **Then** a conflict error is raised or a configurable resolution policy applies

---

### User Story 45 - Tool Operational Controls (Priority: P2)

Developers can configure per-tool operational limits including timeouts, usage caps, enablement predicates, name validation, result truncation, and argument validation. These controls are enforced automatically at the tool execution layer.

**Why this priority**: Production agents need guardrails — timeout prevents hangs, usage limits prevent runaway costs, argument validation prevents errors, and result truncation prevents context overflow.

**Independent Test**: Can be fully tested by creating a tool with a 100ms timeout, invoking it with an operation that takes 500ms, and verifying it returns a timeout error within the configured timeout behaviour.

**Acceptance Scenarios**:

1. **Given** a tool with `timeout: 100ms` and `timeout_behavior: ReturnError`, **When** the tool exceeds 100ms, **Then** execution is cancelled and a timeout error is returned
2. **Given** a tool with `max_usage_count: 3`, **When** invoked a 4th time, **Then** `ToolUsageLimitExceeded` error is returned
3. **Given** a tool with `is_enabled: false`, **When** the LLM schema is generated, **Then** the tool is omitted entirely
4. **Given** a tool with name `invalid name!`, **When** construction is attempted, **Then** name validation rejects it (names must match `^[a-zA-Z0-9_-]{1,64}$`)
5. **Given** a tool result exceeding `max_result_size` (default 100 KB), **When** the result is returned, **Then** `ToolNode` truncates it to the configured limit
6. **Given** a tool with argument schema, **When** arguments fail JSON Schema validation before invocation, **Then** a `SchemaValidation` error is returned without invoking the tool

---

### User Story 46 - Tool Classification and Content Types (Priority: P2)

Developers can classify tools by category (`Builtin`, `Custom`, `Mcp`, `Remote`, `WorkflowAsTool`) and by operational kind (`read`, `edit`, `search`, `execute`, `other`). Tool output carries a content type (`Text`, `Image`, `File`, `Json`) enabling consumers to handle results appropriately.

**Why this priority**: Classification enables permission UIs to communicate tool impact to users. Content types enable downstream processing (e.g., rendering images differently from text).

**Independent Test**: Can be fully tested by creating tools with different categories and kinds, verifying category and kind are correctly reported, and confirming tool output content types are preserved through serialisation.

**Acceptance Scenarios**:

1. **Given** a tool created via `#[tool]` macro, **When** its category is queried, **Then** it reports `ToolCategory::Custom`
2. **Given** a tool loaded from an MCP server, **When** its category is queried, **Then** it reports `ToolCategory::Mcp`
3. **Given** a tool with `ToolKind::edit`, **When** a permission UI queries the tool kind, **Then** it can display appropriate warnings about write operations
4. **Given** a tool returning JSON output, **When** `ToolOutput.content_type` is queried, **Then** it reports `ToolContentType::Json`

---

### User Story 47 - Proc-Macro Tool Generation (Priority: P1)

Developers can generate a full `Tool` implementation from an async function using the `#[tool]` proc-macro, eliminating boilerplate. The macro generates name, description, JSON Schema for parameters, and the invocation wrapper.

**Why this priority**: Custom tool creation is the most common developer task. Reducing boilerplate from ~50 lines to ~5 lines dramatically improves developer experience.

**Independent Test**: Can be fully tested by annotating an async function with `#[tool]`, building, and verifying the generated `Tool` impl has correct name, description, schema, and successfully invokes the underlying function.

**Acceptance Scenarios**:

1. **Given** an async function annotated with `#[tool(description = "...")]`, **When** compiled, **Then** a `Tool` impl is generated with the function name as tool name and the specified description
2. **Given** a `#[tool]` function with typed parameters, **When** the schema is queried, **Then** a correct JSON Schema is generated from the parameter types
3. **Given** a `#[tool]` function returning `Result<ToolOutput, ToolError>`, **When** invoked through the `Tool` trait, **Then** the function is called with deserialised arguments and the result is returned

---

### User Story 48 - Compiled Graph as Tool (Priority: P2)

Developers can wrap a `CompiledGraph` as a `Tool` for graph-in-graph composition, and use a `CompiledGraph` directly as a node within another `StateGraph`. This enables hierarchical agent architectures where complex sub-workflows are encapsulated as tools.

**Why this priority**: Enables building large systems from smaller, tested sub-graphs. A "research agent" can be a tool within a "project manager" agent.

**Independent Test**: Can be fully tested by creating two `CompiledGraph`s, wrapping one as a tool via `as_tool()`, adding it to the other's tool set, and verifying the outer graph can invoke the inner graph as a tool.

**Acceptance Scenarios**:

1. **Given** a `CompiledGraph`, **When** `as_tool()` is called, **Then** the returned `Tool` accepts the graph's input state and returns its output state
2. **Given** a `CompiledGraph` used as a node in another `StateGraph`, **When** the outer graph executes, **Then** the inner graph runs as a subgraph with its own state lifecycle
3. **Given** a graph-as-tool, **When** the inner graph errors, **Then** the error is propagated to the outer graph as a tool error

---

### Edge Cases

- What happens when an agent returns directives but no directive executor is provided? (System records directives without executing — enables pure unit testing)
- How does the system handle plugin state key collisions at runtime? (Compile-time error prevents this via type system — runtime should never encounter collisions)
- What happens when an execution strategy rejects an action as invalid for the current state? (System returns `InvalidTransition` error with current state and attempted action details)
- How does the backend protocol handle operations on paths outside allowed boundaries? (Path traversal protection consistently applied across all backends — rejects with `invalid_path` error)
- What happens when middleware attempts to terminate agent execution early? (Middleware returns `MiddlewareResult::Terminate(AgentResult)` to short-circuit remaining middleware and agent execution)
- How does the system handle exceeding max_turns? (Agent execution stops after configured turn limit with appropriate termination signal)
- What happens when multiple backends in a composite backend can handle the same path? (Longest-match-first semantics — most specific path prefix wins)
- How does the system handle directive serialization round-trips with custom directive variants? (Custom directives implement `DirectivePayload` trait enabling serialization — enables replay and what-if analysis)
- What happens when git operations are attempted outside the scoped repository path? (GitProvider rejects with scope violation error)
- How does the system handle HTTP requests that redirect to different domains? (HttpProvider follows redirects up to configured limit, returns final response)
- What happens when an approval request times out without user response? (Operation fails with timeout error, does not execute)
- How does the system handle binary file uploads/downloads? (Backends handle binary content transparently with content-type detection)
- What happens when git push conflicts with remote changes? (GitProvider returns conflict error with details, does not force-push)
- How does the HTTP backend handle SSL/TLS certificate validation errors? (Returns certificate error, does not proceed with insecure connection)
- What happens when grep search matches thousands of results? (Backend respects max-count limit, returns up to limit with indication more exist)
- How does the system handle killing a process owned by another user? (ProcessProvider returns permission denied error)
- What happens when extracting archive would overwrite existing files? (ArchiveProvider prompts for resolution or uses configured policy: skip/overwrite/rename)
- How does working directory state persist across backend swaps in CompositeProvider? (Each backend maintains its own working directory state independently)
- What happens when a pipeline stage hangs or times out? (Pipeline executor enforces timeout per stage, returns timeout error for hanging stage)
- How does grep handle binary files? (Backend skips binary files by default unless binary-files=text option specified)
- What happens when spawning background job reaches process limit? (ProcessProvider returns resource limit error)
- How does the system handle circular symbolic links during archive creation? (ArchiveProvider detects cycles, returns error or skips with warning based on policy)
- What happens when multiple agents share a Runner concurrently? (Each agent has its own session with isolated state; Runner serializes access to shared resources like checkpoint storage)
- What happens when a subagent outlives its parent? (Orphaned subagents receive a stop signal; if the parent's supervision policy is `transient`, orphans are stopped gracefully; if `permanent`, they are adopted by the Runner)
- What happens with circular subagent spawning (A spawns B spawns A)? (Runner tracks spawn depth and rejects spawns exceeding configurable `max_spawn_depth`, defaulting to 10)
- Can the middleware stack be modified during execution? (No — the middleware stack is frozen at agent build time; modifying it requires building a new agent instance)
- What happens when `RunInstruction` results arrive after the requesting agent has stopped? (Results are discarded; the directive executor checks agent liveness before routing results back)
- What happens when a model API error occurs during streaming after partial events have been emitted? (An `Error` event is emitted after the partial events; consumers must handle partial-then-error sequences; `is_final_response()` returns true for the error event)
- What happens when the minimum model capability (tool-calling) is not supported by the selected model? (Agent builder validates model capabilities at build time; if tool-calling is required but unsupported, `build()` returns an error. For models without tool-calling, agents can operate in prompt-only mode with OutputMode::Prompt)
- How does TOCTOU (time-of-check-time-of-use) affect path validation? (Path validation and file operation occur within the same async task holding the Mutex lock — no gap for symlink replacement. Real filesystem operations use `O_NOFOLLOW` where available)
- Can a subagent have more permissions than its parent? (No — subagents MUST inherit the parent's `SandboxConfig` and `PermissionMode` as upper bounds. A child may be more restrictive but never more permissive)
- How are API keys and auth tokens managed in `HttpProvider` and MCP transports? (Credentials are stored in `secrecy::SecretString`, excluded from `Debug` output, and never serialized to checkpoint state or logs. Backend implementations MUST use the existing `credentials` module)
- What happens when `max_turns` and `max_budget` are both exceeded in the same turn? (`max_budget` is checked first — if cost exceeds budget, `BudgetExceeded` takes priority over `MaxTurnsExceeded`)
- Can `PermissionMode::PlanOnly` interact with sandbox settings? (They are independent — `PlanOnly` prevents mutations via permission checks, `SandboxConfig` restricts what resources are accessible. Both can be set simultaneously for defense-in-depth)
- What happens when `ThinkingConfig::Disabled` is combined with `effort: Max`? (This is a contradiction — `build()` returns an error per FR-701)
- What happens if a plugin's state schema changes between sessions? (Deserialization failure resets that plugin's state to `Default::default()` with a warning, per FR-656. Other plugins and agent state are unaffected)
- What is the initial working directory for each provider type? (The agent's `cwd` config if set, otherwise the provider's root directory. For `CompositeProvider`, each sub-provider starts at its own root. `CompositeProvider.pwd()` returns the composite-level cwd, not individual provider cwds)
- Is the working directory included in checkpoint state? (Yes — per FR-660, working directory per backend is part of the checkpoint)
- What happens if directive filters reject a directive but state was already updated? (The state change is preserved — per FR-665, state update happens before filtering. Only the directive's side effects are prevented)
- Can events from concurrent tool calls interleave? (Yes — per FR-670. Each tool call's internal sequence is preserved but events from different tool calls may interleave. Consumers correlate by tool call ID)
- What happens when `skeleton` is called on a binary file? (Returns `VfsError::Unsupported` — skeleton only applies to text files with tree-sitter grammar support)
- What happens when the code graph encounters a symbol with the same name in multiple files? (Graph nodes are keyed by (file, symbol) tuples — same-named symbols in different files are distinct nodes)
- What happens when hybrid search alpha is outside [0.0, 1.0]? (Clamped to the valid range with a warning log)
- What happens when SBFL coverage data is incomplete (not all functions instrumented)? (Functions without coverage data receive a neutral score of 0.0 — they are neither suspicious nor exonerated)
- What happens when the experience pool grows very large? (Configurable size limit with LRU eviction of oldest associations; file summaries are regenerated on demand)
- What happens when MCTS explores a trajectory that modifies files? (MCTS trajectories are exploratory only — no file modifications are committed until a trajectory is selected and explicitly applied)
- What happens when a dynamically-created tool has the same name as a native tool? (The dynamic tool is rejected with a name collision error — native tools cannot be overridden via `CreateTool`)
- What happens when a Lua/Rhai script enters an infinite loop? (Scripting runtimes MUST enforce a configurable instruction/fuel limit. Lua uses `debug.sethook` with an instruction count limit; Rhai has built-in `max_operations`. Exceeding the limit terminates the script with a timeout error)
- What happens when a WASM skill panics? (Extism isolates the panic within the WASM sandbox. The host receives an error result. No host process crash)
- What happens when a skill manifest requests permissions the agent's SandboxConfig doesn't allow? (The skill is rejected at load time — its tools are not registered, and a warning is logged explaining which permissions were denied)
- What happens when two skills declare the same tool name? (The skill loaded later wins if it has a higher version. If same version, the second is rejected with a name collision error)
- What happens when an external script skill is loaded? (The skill loader emits a warning: "runtime 'external' bypasses embedded sandboxing — prefer lua, rhai, or wasm". The skill is still loaded but the warning is logged and surfaced to the user on first invocation)
- What happens when dataflow analysis encounters dynamic dispatch (trait objects, virtual calls)? (Dataflow traces to the trait method declaration; concrete implementations are listed as possible targets but not definitively resolved)
- What happens when a `graph_search` query matches a node with no edges? (The node itself is returned as a single-node subgraph — no expansion occurs)
- What happens when the LSP server crashes during a `lsp_find_references` call?
- What happens when the code graph is too sparse for meaningful community detection? (Communities degenerate to individual nodes — the system returns single-member communities and community_search falls back to direct semantic search)
- What happens when the resolution parameter produces too many or too few communities? (Resolution is configurable per-index; users can re-run with different resolution without rebuilding the graph. Default chosen for typical code structure)
- What happens when a community summary becomes stale but the LLM is unavailable?
- What happens when `ProjectId` is computed on a shallow clone without the first commit? (Falls back to `sha256(canonical_path)` — shallow clones lack full history. Logs a warning suggesting `git fetch --unshallow` for stable identity)
- What happens when two MCP server instances write to the same project simultaneously? (SQLite WAL serialises writes automatically — the second writer waits briefly for the first to commit, then proceeds. Readers are never blocked. No manual lock management needed)
- What happens when the storage migration fails midway? (Copy-then-swap: the migration operates on a temporary copy, only swapping to the new version on success. The original data is untouched on failure)
- What happens when `$XDG_DATA_HOME` is not set on Linux? (Falls back to `$HOME/.local/share` per XDG Base Directory Specification, via the `directories` crate)
- What happens when the `.synwire/config.json` in a project root conflicts with the environment variable? (Environment variable wins — configuration hierarchy is strictly ordered)
- What happens when the daemon's Unix domain socket file exists but the process is dead (stale socket)? (The MCP server detects the stale socket via connect failure, checks the PID file, confirms the process is dead, removes both files, and starts a new daemon)
- What happens when two MCP servers try to start the daemon simultaneously? (Atomic PID file creation — first writer wins and starts the daemon. The loser detects the PID file, waits briefly for the socket to appear, and connects as a client)
- What happens when a worktree is deleted while its index exists? (The daemon detects the missing worktree root on next file watcher event, marks the `WorktreeId` as stale, and stops watching it. The index persists on disk as cache until garbage-collected)
- What happens when `git worktree add` creates a new worktree for an already-running daemon? (The daemon is unaware until an MCP server for the new worktree connects. At that point, a new `WorktreeId` index is created on demand within the existing daemon)
- What happens when the daemon is managing 10+ repos and memory usage grows? (The daemon uses per-repo LRU eviction for in-memory caches — repos not queried recently have their in-memory state evicted while on-disk indices remain. The embedding model is shared and not evicted)
- What happens when a `clone_repo` targets a repo already registered in the daemon under a different path?
- What happens when `$XDG_DATA_HOME` points to a read-only filesystem? (MCP server fails at startup — same as FR-888t, StorageLayout directory not writable)
- What happens when project-local config `.<product>/config.json` contains invalid JSON? (Config file is skipped with a warning, falling back to CLI flags or platform defaults. The server still starts)
- What happens when SQLite WAL mode cannot be enabled (e.g., NFS or network filesystem)? (Fall back to SQLite journal mode DELETE. Concurrent readers may block during writes. Emit a warning that multi-instance performance will be degraded)
- What happens when a first-run with no existing data occurs? (StorageLayout creates directories lazily on first write. ProjectRegistry starts empty. Daemon starts with no repos registered. All normal — first `index` call populates everything)
- What happens when upgrading from old hardcoded `$CACHE/synwire/indices/<sha256>/` paths to new StorageLayout? (FR-828 migration framework detects the old layout via absence of `version.json` in the new location. One-time migration copies data from old path to new WorktreeId-based path. Old data is not deleted automatically — user runs `storage_gc` to clean up)
- What happens when `tool_search` returns no results? (Returns the `tool_list` output instead — full namespace listing so the LLM can browse and refine its query)
- What happens when a tool's name changes between sessions (e.g., a skill is updated)? (The `ToolSearchIndex` is rebuilt at startup. Stale cached schemas from previous sessions are not reused — each session starts fresh)
- What happens when the LLM tries to invoke a tool whose schema hasn't been loaded via `tool_search`? (The tool still works — `ToolSearchIndex` is an optimisation, not a gate (FR-908). The tool executes normally but the LLM may have incorrect parameter assumptions. A warning is logged)
- What happens when a third-party agent loads all tools without using `ToolSearchIndex`? (Everything works normally, just with higher token usage and potentially lower accuracy. This is the opt-out path)
- What happens when disk space runs out during indexing? (The daemon's indexing pipeline detects write failures from LanceDB/SQLite/tantivy, marks the index as incomplete, and returns an error. Partial index data is not corrupt — the next indexing attempt resumes from the last successful file via content hash registry)
- What happens when `StorageLayout` directories have incorrect permissions? (MCP server fails at startup with a clear error identifying the non-writable directory — FR-888t) (The daemon detects matching `RepoId`, treats the new path as an additional worktree, and reuses the existing repo coordinator state)
- What happens when the global experience pool grows very large across many projects? (Same LRU eviction as project-local pool, with configurable size limit. Old entries from rarely-accessed projects are evicted first)
- What happens when a dependency changes version across projects? (The dependency index stores per-project version constraints — `project A → lib X v1.9`, `project B → lib X v2.0` — both are recorded as separate edges)
- What happens when two agents on different projects both try to write to the global experience pool?
- What happens when a cross-project reference points to a project that is no longer indexed? (The reference becomes a dangling edge — `xref_query` returns it with a `resolved: false` flag. If the project is later re-indexed, the reference resolves automatically)
- What happens when a dependency is available as both a local project and a remote crate?
- What happens when `clone_repo` targets a very large repository (>1GB)? (Default behaviour uses `depth: 1` shallow clone for repos over a configurable size threshold. The user can override with `depth: null` for full history)
- What happens when `clone_repo` fails partway through (network interruption)? (The partial clone directory is removed. A retry starts fresh. The `git clone` process is run with `--no-checkout` first, then checkout, to minimise the corrupted-state window)
- What happens when the agent mounts the same repository twice at different refs? (Each ref gets a separate mount point: `/repos/owner/repo/` for default and `/repos/owner/repo@v2.0/` for a tagged ref. Both are independently searchable)
- What happens when `repo_gc` tries to remove a repository that is currently mounted? (The repository is skipped with a warning — active mounts are never garbage-collected)
- What happens when the `RepoFetchDetector` triggers but the repository is private and no credentials are available?
- What happens when the editor closes while indexing is in progress? (The MCP server process exits, cancelling the in-progress indexing. The partially-built index is incomplete but not corrupt — next startup will detect the incomplete state via missing `meta.json` and re-index)
- What happens when two MCP server instances both try to index the same project simultaneously? (The second instance blocks on the exclusive `ProjectLock` until the first completes. It then detects the fresh index via `meta.json` and skips re-indexing)
- What happens when the MCP server is started without `--project` and a tool requiring a project is called? (Returns an MCP error with a message explaining that `index` must be called first to set a project, or `--project` must be specified at startup)
- What happens when an MCP client calls `write` or `edit` on a file in a cloned repo? (Allowed — the mount is a real `LocalProvider`. The agent can modify files in cloned repos, but changes are local and uncommitted)
- Can the MCP server expose tools from multiple projects simultaneously?
- What happens when the MCP client doesn't support sampling but a tool requires it? (The tool uses the `SamplingProvider` graceful degradation path — FR-884. No error is returned; the feature works with reduced quality)
- What happens when a sampling request times out? (The tool falls back to the no-sampling degradation path for that specific invocation, logs a warning, and returns the degraded result)
- What happens when sampling is used for community summaries on a very large community (100+ members)? (The prompt is truncated to fit the model's context window, including only the top-N most connected members and a count of omitted members)
- What happens when an agent asks for summaries of all communities? (Each `community_summary(id)` call triggers at most 1 sampling call. The agent controls the loop — if it iterates over 500 communities, it makes 500 calls. This is the agent's decision, not the indexer's. Cached summaries avoid repeat calls) (Yes — each `index` or `clone_repo` call adds a mount. The VFS `CompositeProvider` routes by path prefix. All mounted projects are searchable) (The suggestion is still emitted. If the user accepts and `clone_repo` fails with an auth error, the agent reports the error and continues using `web_fetch`) (Local project takes precedence if indexed — the cross-project edge links to the local definition. If the local project is not indexed, the dependency is recorded in the dependency index but no code-level cross-reference is created) (Global `ProjectLock` serialises writes. Reads use shared locks so concurrent reads from different project agents are fine) (The stale summary is returned with a `stale: true` flag; regeneration is retried on next access) (The LspClient returns an error, and if auto-reconnect is enabled, attempts to restart the server for subsequent requests)

**MCP Adapters Edge Cases**:

- What happens when an MCP server disconnects mid-tool-call? (The tool call returns a `Transport` error. The `MultiServerMcpClient` marks the server as unhealthy. Subsequent calls to that server fail immediately until reconnection succeeds)
- What happens when two MCP servers expose tools with the same name? (If `tool_name_prefix` is enabled, no collision — tools are prefixed as `{server}_{name}`. Without prefixing, the second server's tool is rejected with a name collision error)
- What happens when cursor pagination returns inconsistent results (new tools added mid-pagination)? (Pagination proceeds with the data available — the tool list reflects a point-in-time snapshot. The 1000-page cap prevents infinite loops from misbehaving servers)
- What happens when an interceptor modifies the tool arguments? (Modified arguments are passed to inner interceptors and the tool. This is by design — interceptors may inject context, redact secrets, or transform arguments)
- What happens when `to_mcp_tool()` encounters a tool with injected arguments in the schema? (Returns an error — injected arguments are not part of the MCP schema contract)
- What happens when a `ToolProvider` discovers tools concurrently from multiple sources? (`CompositeToolProvider` fetches from all sub-providers concurrently. Results are merged. Name collisions are handled per the configured resolution policy)
- What happens when a tool's `max_usage_count` is reached mid-conversation? (The tool returns `ToolUsageLimitExceeded` for all subsequent calls. The limit resets per-session, not per-conversation, unless configured otherwise)
- What happens when `CompiledGraph::as_tool()` wraps a graph that requires checkpoint support? (The inner graph maintains its own checkpoint state. The outer graph sees the tool as a black box — it does not participate in the outer graph's checkpoint)
- What happens when a WebSocket MCP transport loses connection? (The `McpLifecycleManager` detects the disconnect and attempts reconnection with exponential backoff. During reconnection, tool calls to that server return `ConnectionFailed`)
- What happens when JSON Schema validation fails for a tool argument but the MCP server would accept it? (The client-side validation is strict — the call is rejected before reaching the server. This is intentional to catch errors early and avoid network round-trips for malformed requests)
- What happens when a `#[tool]` macro function has complex generic type parameters? (The macro generates schema from concrete types only — generic functions must be monomorphised before the macro can generate a schema. Type parameters in the function signature cause a compile-time error)
- What happens when MCP LoggingMessage callbacks are received at high frequency? (Callbacks are delivered asynchronously. If the callback handler is slow, messages may queue. The system does not drop messages but imposes a configurable buffer limit, after which oldest messages are dropped with a warning)

## Requirements *(mandatory)*

### Functional Requirements

**Directive System (FR-557–562)**:

- **FR-557**: System MUST provide a `Directive` enum with typed variants including Emit, SpawnAgent, StopChild, Schedule, RunInstruction, Cron, Stop, and support for custom variants via `Directive::Custom`
- **FR-558**: Agent nodes MUST return `DirectiveResult<S>` combining updated state and zero or more directives, with state changes applied immediately and directives deferred to executor
- **FR-559**: System MUST provide `DirectiveExecutor` trait with `execute_directive` method, including default implementation and support for custom executors for testing and dry-run analysis
- **FR-560**: System MUST support `RunInstruction` directive enabling pure agents to request runtime execution where executor runs the action and routes results back as agent input
- **FR-561**: System MUST provide `DirectiveFilter` trait allowing middleware to inspect, transform, or suppress directives before execution for policy enforcement and audit logging
- **FR-562**: All directive variants MUST implement `Serialize`/`Deserialize` enabling recording of agent decisions and replay against different executor implementations

**Execution Strategies (FR-563–567)**:

- **FR-563**: System MUST provide `ExecutionStrategy` trait with `execute`, `tick` for multi-step continuation, and `snapshot` for stable execution views
- **FR-564**: System MUST provide `DirectStrategy` that executes actions immediately and sequentially as default for simple request-response workflows
- **FR-565**: System MUST provide `FsmStrategy` implementing finite state machine with explicit state transitions enforcing "action X only valid in state Y" constraints
- **FR-566**: System MUST support `FsmTransition` type defining from_state, to_state, action, and optional guard condition with builder API, returning `InvalidTransition` error with current state and attempted action for invalid transitions
- **FR-567**: Execution strategies MUST support strategy-level signal routing via `signal_routes()` method returning priority-ordered route mappings

**Plugin System (FR-143–144, FR-568–570)**:

- **FR-143**: System MUST support runner-scoped plugins with `on_user_message`, `on_event`, `before_run`, and `after_run` hooks
- **FR-144**: Plugins MUST declare hooks that execute at appropriate lifecycle points
- **FR-568**: Each plugin MUST declare a `PluginStateKey` with associated `State` type and unique key string, with plugin state nested under the key in agent state
- **FR-569**: System MUST provide type-safe state accessors `plugin_state<P: PluginStateKey>() -> &P::State` and `plugin_state_mut<P>() -> &mut P::State` preventing cross-plugin state access without explicit interface
- **FR-570**: When multiple plugins are composed, system MUST merge their state schemas automatically and produce compile-time error for conflicting state keys

**Signal Routing (FR-571–572)**:

- **FR-571**: System MUST implement three-tier signal routing priority: (1) execution strategy routes, (2) agent-level routes, (3) plugin-contributed routes, with first match winning
- **FR-572**: System MUST provide `SignalRouter` trait with `route(&self, signal: &Signal) -> Option<Action>` method, composed from strategy, agent, and plugin routers, with routing decisions logged at debug level

**Backend Protocol (FR-070–074)**:

- **FR-070**: System MUST provide `Vfs` trait defining pluggable interface for file operations (`ls_info`, `read`, `write`, `edit`, `grep_search`, `glob_info`, `upload_files`, `download_files`, `pwd`, `cd`) with both sync and async variants
- **FR-070a**: System MUST provide enhanced `grep_search` with `GrepOptions` supporting context lines (before/after/both), line numbers, case sensitivity, invert match, count mode, files-only mode, file type filtering, max match limit, and binary file handling
- **FR-070b**: All backends MUST maintain persistent working directory state across operations, with `cd` changing state and file operations resolving relative paths from current directory
- **FR-071**: System MUST provide `SandboxProtocol` extending `Vfs` with shell execution (`execute`/`aexecute`), pipeline execution (`execute_pipeline`), stream redirection, and unique `id` property
- **FR-071a**: System MUST support command pipelines enabling output of one command to pipe to input of next command with proper error propagation
- **FR-071b**: System MUST support stream redirection for stdin, stdout, and stderr with ability to redirect to files, combine streams, or provide input from strings
- **FR-072**: System MUST define backend response types including `WriteResult`, `EditResult`, `ExecuteResponse`, `FileInfo`, `GrepMatch` (with line numbers and context), `FileDownloadResponse`, `FileUploadResponse`, `ProcessInfo`, `JobInfo`, `ArchiveInfo`
- **FR-072a**: GrepMatch responses MUST include line number, column offset, matched text, and configurable context lines before/after match
- **FR-073**: System MUST provide standardized `VfsError` with error codes: `file_not_found`, `permission_denied`, `is_directory`, `invalid_path`, `scope_violation`, `resource_limit`, `timeout`
- **FR-074**: System MUST support `VfsFactory` type alias enabling late-binding backend construction at graph compile time
- **FR-074a**: System MUST support bash-style command conventions (ls, cd, grep, rm, cp, mv, pwd, ps, kill, tar, gzip) that translate to appropriate backend protocol operations, maintaining familiar shell semantics

**VFS Provider Implementations (FR-075–080, FR-081a–081f)** [FR-081a–081f SUPERSEDED by FR-724]:

- **FR-075**: System MUST provide `MemoryProvider` for ephemeral per-conversation file storage in agent state with `files_update` dicts for checkpointing
- **FR-076**: System MUST provide `StoreProvider` for persistent cross-conversation storage via `BaseStore` with configurable namespace isolation
- **FR-077**: System MUST provide `LocalProvider` with virtual mode with path traversal protection and real mode, including symlink traversal protection
- **FR-078**: System MUST provide `LocalShellProvider` extending `LocalProvider` with shell execution, environment variable control, output truncation, and timeout
- **FR-079**: System MUST provide `CompositeProvider` routing file operations to sub-backends by path prefix using longest-match-first semantics, with aggregated listings and cross-backend search
- **FR-080**: System MUST provide abstract `BaseSandbox` type implementing all `Vfs` operations by delegating to `execute()`, requiring subclasses to implement only `execute()`, `upload_files()`, `download_files()`, and `id`
- **FR-081a** [SUPERSEDED by FR-724]: ~~System MUST provide `GitProvider` for version control operations~~ with methods for status, diff, log, commit, push, pull, branch management, scoped to specific repository paths
- **FR-081b** [SUPERSEDED by FR-724]: ~~System MUST provide `HttpProvider` for web requests~~ with methods for GET, POST, PUT, DELETE, custom HTTP methods, header management, timeout configuration, and redirect following
- **FR-081c** [SUPERSEDED]: ~~GitProvider MUST reject~~ operations outside scoped repository path with scope violation error
- **FR-081d** [SUPERSEDED]: ~~HttpProvider MUST support~~ configurable timeout, SSL/TLS certificate validation, custom headers, and request/response body handling for both text and binary content
- **FR-081e** [SUPERSEDED]: ~~GitProvider MUST return~~ structured responses including commit hashes, author information, timestamps, branch names, and diff output in unified format
- **FR-081f** [SUPERSEDED]: ~~HttpProvider MUST return~~ structured responses including status code, headers, body content, final URL (after redirects), and detailed error information for network failures
- **FR-081g**: System MUST provide `ProcessProvider` for process management with methods for list_processes, kill_process, spawn_background, list_jobs, foreground_job, background_job
- **FR-081h**: ProcessProvider MUST return process information including PID, command line, CPU usage, memory usage, parent PID, and process state
- **FR-081i**: System MUST provide `ArchiveProvider` for archive operations with methods for create_archive, extract_archive, list_contents supporting tar, gzip, zip, bzip2 formats
- **FR-081j**: ArchiveProvider MUST support compression levels, preservation of file permissions/ownership, and configurable conflict resolution policies (skip/overwrite/rename/error)
- **FR-081k**: ArchiveProvider MUST detect and handle circular symbolic links during archive creation returning error or skipping based on policy
- **FR-081l**: All backends MUST support environment variable operations (get, set, unset, list) enabling agents to read and modify process environment

**Approval & HITL (FR-082a–082d)**:

- **FR-082a**: System MUST provide `ApprovalCallback` trait with methods for requesting approval, providing operation context, and receiving user response. The trait applies to all tool invocations (not only backend operations) when a permission rule or mode requires approval
- **FR-082b**: Backends MUST support configurable approval gates via `approval_required` flag or predicate function determining which operations require approval
- **FR-082c**: Approval requests MUST include operation type, affected resources (file paths, URLs, repositories), operation description, and risk level
- **FR-082d**: System MUST support approval timeout configuration — timeout MUST result in denial (not silent queue), returning `ApprovalTimeout` error to the agent

**Middleware Stack (FR-083–093)**:

- **FR-083**: System MUST provide stackable, ordered middleware that can add tools, modify system prompts, and transform state, with default stack order
- **FR-084**: System MUST provide `FilesystemMiddleware` exposing backend file operations as agent tools including enhanced grep with all options
- **FR-085**: System MUST provide `GitMiddleware` exposing git operations as agent tools with automatic repository detection
- **FR-086**: System MUST provide `HttpMiddleware` exposing web request capabilities as agent tools with URL validation
- **FR-087**: System MUST provide `ProcessMiddleware` exposing process management operations as agent tools with safety guards
- **FR-088**: System MUST provide `ArchiveMiddleware` exposing archive/compression operations as agent tools with format detection
- **FR-089**: System MUST provide `SummarisationMiddleware` triggering context summarization based on configurable thresholds: message count, token count, or context window utilization percentage (e.g., compact at 80% utilization)
- **FR-090**: System MUST provide `PatchToolCallsMiddleware` detecting dangling tool calls and adding synthetic `ToolMessage` responses
- **FR-091**: System MUST provide `PromptCachingMiddleware` for provider-specific prompt caching
- **FR-092**: System MUST provide `PipelineMiddleware` exposing command pipeline composition as agent tools with stream handling
- **FR-093**: System MUST provide `EnvironmentMiddleware` exposing environment variable operations as agent tools with scoping

**Convenience API (FR-133–138)**:

- **FR-133**: System MUST provide `Agent<D, O>` struct with builder API for typed dependencies, structured output, and automatic output mode negotiation
- **FR-134**: System MUST provide `RunContext<D>` carrying typed dependencies, model reference, retry count, usage, and metadata
- **FR-135**: System MUST provide `OutputMode<T>` enum with variants Tool, Native, Prompt, and Custom, with Tool as default universal mode
- **FR-136**: System MUST support `ToolResult::Retry(String)` enabling tool-initiated model self-correction with a maximum of 3 retries per tool invocation (configurable), with the retry message appended to context for the model's next attempt
- **FR-137**: System MUST provide `ModelSelector` supporting `by_name`, `by_provider`, and `by_capability` constructors, where capabilities queryable via `by_capability` include: tool-calling, vision, streaming, structured output, and effort level support
- **FR-138**: System MUST provide `AgentNode` trait defining `name()`, `description()`, `run()` returning `Stream<AgentEvent>`, and `sub_agents()` methods

**Agent Callbacks (FR-139)**:

- **FR-139**: System MUST provide `BeforeAgentCallback` and `AfterAgentCallback` for per-agent observability

**Execution Control (FR-363–366)**:

- **FR-363**: System MUST support `max_turns: Option<u32>` defaulting to 10 to limit model invocation cycles. When the limit is reached, the agent MUST emit a `TurnComplete` event with a `max_turns_exceeded` reason and return an `AgentResult` with the current state (not an error)
- **FR-364**: System MUST support `run_error_handlers` using `RunErrorAction` with Continue, Retry, and Abort variants
- **FR-365**: System MUST support `tool_error_formatter` allowing custom formatting of tool errors before returning to LLM
- **FR-366**: Middleware MUST support early termination via `MiddlewareResult::Terminate(AgentResult)`

**Streaming Events (FR-157–159, FR-157a–157f)**:

- **FR-157**: System MUST distinguish streaming partial events from final complete events. The `AgentEvent` enum MUST include at minimum: `TextDelta`, `ToolCallStart`, `ToolCallDelta`, `ToolCallEnd`, `ToolResult`, `ToolProgress`, `StateUpdate`, `DirectiveEmitted`, `StatusUpdate`, `UsageUpdate`, `RateLimitInfo`, `TaskNotification`, `PromptSuggestion`, `TurnComplete`, `Error`
- **FR-157a**: `StatusUpdate` events MUST be emitted for long-running internal operations (context compaction, permission pending, model switching) with a status string and optional progress percentage
- **FR-157b**: `RateLimitInfo` events MUST be emitted when the model provider returns rate-limit headers, including utilization percentage, reset time, and whether the request was allowed or rejected
- **FR-157c**: `TaskNotification` events MUST cover background task lifecycle: started (with ID and description), progress (with message), completed (with result), and failed (with error)
- **FR-157d**: `PromptSuggestion` events MAY be emitted after turn completion, providing AI-generated follow-up prompt suggestions for the consumer
- **FR-157e**: `StructuredOutput` MUST be included in the final `AgentResult` when a JSON Schema output format is configured, containing the parsed and validated output alongside the raw text
- **FR-157f**: `ToolProgress` events MUST be emitted during long-running tool execution, carrying partial output text and an optional progress percentage
- **FR-158**: System MUST emit `turn_complete` signal when agent invocation finishes, including termination reason (complete, max_turns_exceeded, budget_exceeded, stopped, error)
- **FR-159**: System MUST provide `is_final_response()` logic to identify final events — returns true for `TurnComplete`, `Error`, and any event carrying a termination reason

**Graceful & Force Stop (FR-164–165)**:

- **FR-164**: System MUST support graceful stop: drain in-flight tool executions (wait for completion), execute pending directive filters, emit `TurnComplete` with reason `stopped`, and propagate stop signal to child agents
- **FR-165**: System MUST support force stop (abort): immediately cancel all in-flight operations via cooperative cancellation signal, skip pending directives, emit `TurnComplete` with reason `aborted`, and force-stop all child agents

**Runner (FR-160–163)**:

- **FR-160**: System MUST provide Runner managing session lookup, routing, invocation, and event collection
- **FR-161**: Runner MUST route requests to appropriate agent sessions
- **FR-162**: System MUST support error recovery via `OnModelErrorCallback` with optional substitute response
- **FR-163**: System MUST support `skip_summarization` flag preventing summarization for technical errors

**Session Management (FR-573–579)**:

- **FR-573**: System MUST support resuming a previously-persisted agent session by session ID, restoring state, conversation history, and plugin state from checkpoint storage
- **FR-574**: System MUST provide session enumeration with `list_sessions()` returning session metadata (ID, creation time, last modified, summary) with optional filtering by workspace or tag
- **FR-575**: System MUST provide `delete_session(id)` for permanent removal of session data from checkpoint storage
- **FR-576**: System MUST provide `get_session_info(id)` returning session metadata without loading full conversation state
- **FR-577**: System MUST support session forking via `fork_session(id, at_message)` creating a divergent copy branched at a specific message, preserving history up to that point
- **FR-578**: System MUST support state rewind via `rewind_to(message_id)` rolling back agent state and file changes to a prior checkpoint, reporting affected files and change counts
- **FR-579**: System MUST support session metadata operations: `tag_session(id, tag)`, `rename_session(id, title)` for organizational discovery

**Hook System (FR-580–590)**:

- **FR-580**: System MUST provide `HookRegistry` supporting registration of typed hook callbacks at lifecycle points, with hooks executing in registration order
- **FR-581**: System MUST provide `PreToolUse` hook executing before tool invocation, receiving tool name and arguments, returning one of: approve (proceed), reject (return error to model), modify (change tool arguments before execution)
- **FR-582**: System MUST provide `PostToolUse` hook executing after successful tool invocation, receiving tool name, arguments, and result, optionally returning modified result
- **FR-583**: System MUST provide `PostToolUseFailure` hook executing after failed tool invocation, receiving tool name, arguments, and error, optionally returning recovery action
- **FR-584**: System MUST provide `Notification` hook for observing system events (model calls, state changes, compaction) without modification capability
- **FR-585**: System MUST provide `SubagentStart` and `SubagentStop` hooks for observing child agent lifecycle events including spawn configuration and termination reason
- **FR-586**: System MUST provide `PreCompact` and `PostCompact` hooks for observing context compaction events, with `PreCompact` receiving current token usage and `PostCompact` receiving compaction summary
- **FR-587**: System MUST support hook timeout configuration per hook registration, with timeout resulting in hook skip and warning log (not agent failure)
- **FR-588**: System MUST support hook matcher patterns scoping hooks to specific tools by name pattern (e.g., `"fs_*"` matches all filesystem tools), with unmatched hooks not invoked
- **FR-589**: Session lifecycle hooks MUST include session source (new, resume, fork) in `SessionStart` and termination reason (complete, error, abort, timeout) in `SessionEnd`
- **FR-590**: Hook callbacks MUST receive an abort signal enabling cooperative cancellation when the agent is stopping

**Model & Provider Management (FR-591–597)**:

- **FR-591**: System MUST provide `list_models()` returning available models with capability metadata (supports vision, supports tool-calling, supports streaming, supported effort levels, context window size, max output tokens)
- **FR-592**: System MUST support dynamic model switching mid-conversation via `set_model(model_id)` without restarting the session, effective from the next turn
- **FR-593**: System MUST support reasoning effort configuration via `effort` parameter with levels Low, Medium, High, Max, controlling model reasoning depth where supported by the provider
- **FR-594**: System MUST support fallback model configuration via `fallback_model` — when the primary model returns an unavailability or rate-limit error, automatically retry with the fallback
- **FR-595**: System MUST track token usage per turn and cumulatively: input tokens, output tokens, cache read tokens, cache creation tokens, and provide estimated cost in USD via `Usage` struct
- **FR-596**: `ModelSelector::by_capability` MUST support querying for: tool-calling, vision, streaming, structured output, and effort level support
- **FR-597**: System MUST support thinking/reasoning budget configuration via `ThinkingConfig` with modes: adaptive (model decides), enabled (fixed token budget), disabled (no reasoning)

**MCP Integration (FR-598–603)**:

- **FR-598**: System MUST support connecting to external MCP servers as dynamic tool providers, with tools from MCP servers automatically available to the agent alongside native tools
- **FR-599**: System MUST support multiple MCP transport types: stdio (subprocess), HTTP (streamable), SSE (server-sent events), and in-process (embedded)
- **FR-600**: System MUST provide MCP server lifecycle management: connect on agent start, automatic reconnect on transient failure, health status monitoring, and runtime disable/enable per server
- **FR-601**: System MUST support MCP elicitation — MCP servers MAY request user input through the agent via an `OnElicitation` callback, with the agent host relaying the request and returning the user's response
- **FR-602**: System MUST support creating in-process MCP servers from tool definitions, enabling tools to be exposed via MCP protocol without a subprocess
- **FR-603**: MCP server status MUST be queryable at runtime, returning per-server: connection state (connected, failed, pending, disabled), server info (name, version), available tools with annotations, and error details if failed

**Tool Configuration (FR-604–609)**:

- **FR-604**: System MUST support overriding built-in tools with custom implementations by registering a tool with the same name and an `overrides_built_in: true` flag
- **FR-605**: System MUST support per-session tool allow/deny lists: `allowed_tools` (whitelist — only these tools available) and `excluded_tools` (blacklist — all tools except these)
- **FR-606**: System MUST emit tool execution progress events during long-running tool invocations, enabling consumers to show incremental output before the tool completes
- **FR-607**: The `#[tool]` derive macro MUST generate JSON Schema for tool parameters from Rust struct definitions, with schema used for input validation before handler invocation
- **FR-608**: Tools MUST support annotations (`read_only`, `destructive`, `open_world`) as metadata informing permission decisions and UI display
- **FR-609**: Tool result types MUST support: text result (string), binary results (bytes + MIME type), result status (success, failure, rejected, denied), error detail, and telemetry metadata

**System Prompt Management (FR-610–612)**:

- **FR-610**: System MUST support system prompt configuration in two modes: append (additions composed after a base prompt) and replace (complete override of the system prompt)
- **FR-611**: Middleware system prompt contributions MUST be composed in middleware stack order, with each middleware's `system_prompt_additions()` appended in declared order after the base prompt
- **FR-612**: Subagent prompt configuration MUST support per-agent prompt overrides via the `SpawnAgent` directive configuration, independent of the parent agent's prompt

**Permission Modes & Rules (FR-613–616)**:

- **FR-613**: System MUST provide named permission mode presets: `Default` (prompt for dangerous operations), `AcceptEdits` (auto-approve file modifications), `PlanOnly` (read-only, no mutations), `BypassAll` (auto-approve everything), `DenyUnauthorized` (deny if no pre-approved rule matches)
- **FR-614**: System MUST support declarative per-tool permission rules in configuration (not only runtime callbacks), specifying tool name patterns and their permission behavior (allow, deny, ask)
- **FR-615**: `ApprovalCallback` MUST support returning modified tool input alongside the approval decision, enabling approvers to edit tool arguments before execution proceeds
- **FR-616**: `ApprovalDecision` MUST include variants: `Allow`, `Deny`, `AllowAlways` (persistent approval for matching pattern), and `Abort` (deny and stop the entire agent)

**Usage & Cost Tracking (FR-617–619)**:

- **FR-617**: System MUST support `max_budget: Option<f64>` (USD) as an execution limit — agent stops with a `BudgetExceeded` error when cumulative estimated cost exceeds the budget
- **FR-618**: `Usage` struct MUST track per-turn and cumulative: input tokens, output tokens, cache read tokens, cache creation tokens, estimated cost USD, and context window utilization percentage
- **FR-619**: System MUST emit `UsageUpdate` events after each model invocation, enabling consumers to monitor token consumption and cost in real time

**Agent Sandbox & Debug (FR-620–622)**:

- **FR-620**: System MUST support agent-level sandbox configuration: network isolation (allow/deny outbound connections), filesystem scope (restrict to specific directories), and command restrictions (allow/deny specific shell commands)
- **FR-621**: System MUST support debug mode via `debug: bool` configuration, emitting verbose internal state events (strategy transitions, routing decisions, directive processing, middleware chain) when enabled
- **FR-622**: Debug output MUST be directable to a file via `debug_file: Option<PathBuf>` for post-mortem analysis without polluting the event stream

**Background Tasks (FR-623–625)**:

- **FR-623**: System MUST support spawning background tasks via `SpawnTask` directive, returning a task ID for tracking
- **FR-624**: Background tasks MUST emit lifecycle events: `TaskStarted { id, description }`, `TaskProgress { id, message }`, `TaskCompleted { id, result }`, `TaskFailed { id, error }`
- **FR-625**: System MUST support cancelling background tasks by ID via `StopTask` directive, with the task receiving a cooperative cancellation signal

**Structured Output (FR-626)**:

- **FR-626**: System MUST support JSON Schema-based structured output configuration, where the agent's final result is parsed and validated against the schema, returned as typed `structured_output` alongside the raw text result

**Error Taxonomy (FR-627–629)**:

- **FR-627**: System MUST define error categories consistently across all subsystems with a top-level `AgentError` enum containing variants for: model errors (with subtypes: authentication, billing, rate-limit, server, invalid-request), tool errors, strategy errors, middleware errors, directive errors, backend errors, and session errors
- **FR-628**: Each error variant MUST declare whether it is retryable, and `RunErrorAction` dispatch MUST respect this — model rate-limit errors default to Retry, authentication errors default to Abort, tool errors default to Continue
- **FR-629**: Panic recovery in Runner MUST: catch the panic, log the panic payload and backtrace at error level, emit an `Error` event, and return an `AgentError::Panic` to the caller — not silently continue

**Configuration Scoping (FR-630–632)**:

- **FR-630**: System MUST support layered configuration with precedence: runtime options (highest) > per-session config > per-workspace config > global defaults (lowest)
- **FR-631**: Agent-level environment variable configuration MUST be supported via `env: HashMap<String, String>` on the agent builder, passed to backends and shell execution
- **FR-632**: Agent-level working directory MUST be configurable via `cwd: Option<PathBuf>` on the agent builder, used as the initial working directory for all backends

**Defaults & Invariants (FR-633–654)**:

- **FR-633**: The default `PermissionMode` for an unconfigured agent MUST be `Default` — tools classified as dangerous or critical require explicit approval; safe and moderate tools are auto-approved
- **FR-634**: The default `ApprovalDecision` for backends without a configured `ApprovalGate` MUST be auto-approve (no gate = no approval required). An explicit gate is opt-in
- **FR-635**: Default risk level mappings MUST be: read/ls/glob/pwd/grep = `Safe`, write/edit/cp/mv/upload = `Moderate`, rm/execute/push/kill = `Dangerous`, format/drop/force-push = `Critical`. Backends MAY override per-operation
- **FR-636**: `AllowAlways` pattern matching MUST use glob syntax against the full tool name (e.g., `"fs_*"` matches `fs_read`, `fs_write`). Exact match when no glob metacharacters are present
- **FR-637**: `PermissionMode::DenyUnauthorized` MUST deny any tool invocation not covered by at least one `PermissionRule` with `Allow` behavior — the default for unmatched tools is explicit deny
- **FR-638**: Path traversal protection MUST be applied consistently by all filesystem-accessing backends (`LocalProvider`, `GitProvider`, `ArchiveProvider`, `LocalShellProvider`) — each MUST call the shared `resolve()` function that canonicalizes then validates `starts_with(root)`, including symlink resolution
- **FR-639**: `CompositeProvider` MUST validate that no mount prefix is a prefix of another mount on segment boundaries at construction time, preventing ambiguous routing. Mount prefixes MUST be normalized (leading `/`, no trailing `/`)
- **FR-640**: `SandboxConfig` filesystem scope and `Vfs` path traversal protection are layered: `SandboxConfig` defines the allowed directory set, `Vfs` enforces path traversal within each allowed directory. Both checks MUST pass for an operation to proceed
- **FR-641**: `LocalShellProvider` MUST prevent command injection by using `Command::new` with argument arrays (not shell string interpolation). User-provided arguments MUST NOT be passed through a shell interpreter. The shell used for `execute` MUST be configurable (default: `sh -c`) with commands passed as a single argument
- **FR-642**: `SandboxConfig.denied_commands` MUST match against the command name (first token, not full command line) using exact match. Denied command check MUST occur before the command is passed to the OS
- **FR-643**: Pipeline stages MUST each individually pass command restriction checks — a restricted command cannot be accessed by piping through an unrestricted command
- **FR-644**: `LocalShellProvider` output truncation MUST be a hard limit (default: 1MB) applied to both stdout and stderr independently. Truncated output MUST include a `[truncated at {limit} bytes]` suffix
- **FR-645**: `PermissionMode::BypassAll` MUST skip both tool-level permission checks AND backend-level `ApprovalGate` callbacks — it is a complete bypass for development/testing only
- **FR-646**: Approval requests for large tool inputs (>10KB) MUST include a summary (first 200 chars + byte count) rather than the full content
- **FR-647**: `ApprovalDecision::Abort` MUST stop the current agent AND propagate stop to all child agents and background tasks
- **FR-648**: `RunInstruction` directives MUST pass through the `ApprovalGate` if the instruction involves a risky operation (determined by the executor, not the agent). The executor MUST classify the instruction's risk level before execution
- **FR-649**: No default `DirectiveFilter` policy is applied — the filter chain is empty by default. The `SandboxFilter` (blocking SpawnAgent in sandboxed mode) is an opt-in middleware, not a default
- **FR-650**: `DirectiveFilter::Reject` MUST surface the rejection reason to the model as a synthetic tool error message, enabling the model to adjust its approach. `Suppress` MUST be silent (no model feedback)
- **FR-651**: Directive emission MUST be rate-limited per turn: maximum 100 directives per `execute` call (configurable via `max_directives_per_turn`). Exceeding the limit MUST return an error, not silently drop
- **FR-652**: `SpawnTask` directives MUST inherit the parent agent's `SandboxConfig` and `max_budget` constraints. Background tasks MUST NOT have more resources than their spawning agent
- **FR-653**: `Custom` directive variants MUST pass through `DirectiveFilter` and `ApprovalGate` identically to built-in variants — the extensibility mechanism does not bypass safety checks
- **FR-654**: `SandboxConfig.network` isolation MUST default to allow-all when sandbox is disabled, and deny-all (except configured allowlist) when sandbox is enabled. `HttpProvider` MUST respect the sandbox network rules. MCP server connections MUST only connect to servers declared in the agent's `mcp_servers` configuration — no dynamic server discovery

**Checkpoint & State Semantics (FR-655–668)**:

- **FR-655**: Plugin state and agent top-level state `S` are separate namespaces — a plugin's `on_event` hook receives `&AgentState<S>` which provides read access to both `state: &S` and `plugin_state::<P>()`, but mutation is only allowed on the plugin's own state slice via `plugin_state_mut::<P>()`
- **FR-656**: Plugin state schema migration MUST be handled by the `PluginStateKey::State` type's `Deserialize` implementation — if deserialization fails on session resume, the plugin state MUST be reset to `Default::default()` with a warning log, not a hard error
- **FR-657**: `PluginStateMap` serialization format MUST be a flat JSON object keyed by `PluginStateKey::KEY` strings: `{"plugin_a": {...}, "plugin_b": {...}}`
- **FR-658**: Plugin state initialization MUST occur in registration order. Plugins MUST NOT depend on other plugins' state during `Default::default()` initialization
- **FR-659**: `plugin_state_mut` MUST be exclusive per turn — only one middleware/node may hold a mutable reference at a time, enforced by the borrow checker (single `&mut AgentState` per node execution). Concurrent access from different turns is prevented by the Runner's single-turn-at-a-time execution model
- **FR-660**: Checkpoints MUST be written after each completed turn (after directive execution and event emission). A checkpoint MUST include: agent state `S`, plugin state map, conversation history, working directory per backend, FSM current state (if applicable), cumulative `Usage`, and session metadata
- **FR-661**: Checkpoint writes MUST be atomic — if the process crashes mid-write, the previous checkpoint MUST remain valid. Implementation MUST use write-to-temp-then-rename or equivalent atomic strategy
- **FR-662**: Session fork MUST deep-clone all state: agent state, plugin state, conversation history, working directory, FSM state. The fork is fully independent — no shared mutable state with the original
- **FR-663**: Session rewind MUST restore: agent state, plugin state, conversation history, and working directory to the checkpoint at the specified message. Messages after the rewind point MUST be discarded
- **FR-664**: Checkpoint storage MUST enforce a configurable size limit (default: 100MB per session). If a checkpoint exceeds the limit, the write MUST fail with `SessionError::CheckpointTooLarge` — the agent can continue but the session is no longer persistable
- **FR-665**: State mutation order within a turn MUST be: (1) node function produces `DirectiveResult<S>` — state `S` is updated, (2) directive filters run — may suppress/reject directives but MUST NOT modify state, (3) directive executor runs — may produce side effects but MUST NOT modify agent state, (4) events are emitted. If a directive filter rejects, the state change from step 1 is already applied and is NOT rolled back — the rejection only prevents the directive's side effects
- **FR-666**: Middleware state transformation (via `MiddlewareResult`) and directive state updates (`DirectiveResult.state`) operate on different execution phases — middleware transforms input before the node runs, directives transform output after the node runs. They cannot conflict on the same state field within the same phase
- **FR-667**: `PostToolUse` hooks fire AFTER the tool result has been recorded in conversation history but BEFORE the model sees the result. Hook modifications to the result MUST be reflected in what the model receives
- **FR-668**: Child agent `Usage` MUST accumulate into the parent's cumulative `Usage`. A child's token consumption counts against the parent's `max_budget`. Each child also tracks its own `Usage` independently for reporting

**Streaming Protocol Semantics (FR-669–682)**:

- **FR-669**: For a single tool call, events MUST arrive in strict order: `ToolCallStart` → zero or more `ToolCallDelta` → `ToolCallEnd` → zero or more `ToolProgress` → `ToolResult`. No other events for the same tool call ID may appear out of this sequence
- **FR-670**: When multiple tool calls execute concurrently, their event sequences MAY interleave — events from tool call A may appear between events from tool call B. Each tool call's internal sequence (FR-669) MUST be preserved. Consumers MUST use the tool call ID to correlate events
- **FR-671**: `UsageUpdate` MUST be emitted after each model invocation completes and before the next node execution begins. If the turn ends, `UsageUpdate` MUST precede `TurnComplete`
- **FR-672**: `DirectiveEmitted` events MUST fire after `StateUpdate` for the same turn — the state is updated first, then directives are announced
- **FR-673**: Exactly one terminal event (`TurnComplete` or `Error`) MUST be emitted per agent run. It MUST be the last event in the stream. After the terminal event, the stream MUST be closed — no further events are delivered
- **FR-674**: On error-after-partial (model API error during streaming): partial `TextDelta` events already emitted MUST be preserved (not retracted). A synthetic `Error` event MUST follow. Consumers SHOULD treat the partial text as incomplete. If `ToolCallDelta` events were in progress, a synthetic `ToolCallEnd` with an error flag MUST be emitted before the `Error` event
- **FR-675**: `Error` event payload MUST include: error category (model/tool/middleware/etc.), error message, the operation that was in progress when the error occurred, and whether the error is retryable
- **FR-676**: After force-stop (`Aborted`), no `ToolProgress` events from cancelled tools may arrive — the stream is sealed at the `TurnComplete` event. Any late events from cancelled tasks MUST be discarded by the Runner
- **FR-677**: `TerminationReason` variants MUST have documented consumer guidance: `Complete` = all data is trustworthy, `MaxTurnsExceeded` = partial work may exist, `BudgetExceeded` = same as MaxTurnsExceeded, `Stopped` = in-flight tools completed gracefully, `Aborted` = in-flight tools were cancelled (results untrustworthy), `Error` = last operation failed
- **FR-678**: `PromptSuggestion` events MUST arrive AFTER `TurnComplete` — they are post-completion hints. Since FR-673 says TurnComplete is the last event, prompt suggestions MUST be delivered via a separate channel (e.g., included in `TurnComplete` payload) not as standalone events
- **FR-679**: `StatusUpdate.status` MUST use a defined set of values: `"compacting"`, `"permission_pending"`, `"model_switching"`, `"reconnecting_mcp"`, `"thinking"`. The set is `#[non_exhaustive]` — new values may be added
- **FR-680**: `RateLimitInfo` fields: `utilization_pct` is `f32` in range 0.0–1.0, `reset_at` is `Option<DateTime<Utc>>` (ISO 8601 when serialized), `allowed` is `bool`
- **FR-681**: `TaskNotification.kind` MUST use the closed set: `Started`, `Progress`, `Completed`, `Failed`. The set is NOT extensible (unlike `AgentEvent` which is `#[non_exhaustive]`)
- **FR-682**: `AgentEvent` serde format MUST be `#[serde(tag = "type", content = "data")]` — externally tagged with type discriminant and content payload. Consumers encountering unknown `type` values MUST skip the event (forward compatibility with `#[non_exhaustive]`)

**Stream Consumer Contract (FR-683–686)**:

- **FR-683**: The stream type returned by `AgentNode::run` MUST be `BoxStream<'a, Result<AgentEvent, AgentError>>` — errors are delivered out-of-band via `Result::Err` for transport-level failures (stream broken), while application-level errors are delivered in-band via `AgentEvent::Error`. A `Result::Err` terminates the stream without a `TurnComplete`
- **FR-684**: Back-pressure MUST be handled by the `Stream` trait's demand-driven semantics — the Runner only produces the next event when the consumer polls. If the consumer is slow, the agent blocks. No buffering or event dropping
- **FR-685**: Stream reconnection is NOT supported — each `run()` call produces a new stream. To resume after disconnection, the consumer MUST call `run()` again (which starts a new turn). Replaying events from a prior turn requires reading conversation history via `SessionManager`
- **FR-686**: Consumers MUST handle unknown `AgentEvent` variants by skipping them (log at debug level). The `#[non_exhaustive]` attribute guarantees that `match` arms require a wildcard. This is the forward-compatibility contract

**Testing Infrastructure (FR-687–698)**:

- **FR-687**: The boundary between "agent code" (pure, side-effect-free) and "executor code" (performs effects) is defined by trait: code implementing `AgentNode::run` and returning `DirectiveResult` is agent code. Code implementing `DirectiveExecutor`, `Vfs`, `McpTransport`, and `ApprovalGate` is executor code. SC-097 applies only to agent code
- **FR-688**: The 80% coverage target (SC-018) MUST be measured as line coverage across `synwire-core` (new agent modules) and `synwire-agent` combined. `synwire-test-utils` is excluded. Coverage MUST be measured per-workspace aggregate, not per-crate — one crate at 95% cannot mask another at 50%
- **FR-689**: SC-026 ("5 or fewer lines") counts statements in the `Agent::builder()` chain: the builder instantiation, each chained method call on the same expression, and the final `.build()?.run().await?`. Import statements, `#[tokio::main]`, `fn main()`, and `Ok(())` are excluded. Method chains on a single expression count as one statement regardless of line breaks
- **FR-690**: `synwire-test-utils` MUST provide `FakeModel` with configurable responses: sequence of text responses, tool call responses, rate-limit errors, authentication errors, and streaming token sequences. `FakeModel` MUST implement `BaseChatModel` and support `bind_tools`
- **FR-691**: `synwire-test-utils` MUST provide `InProcessMcpServer` — a test MCP server that runs in-process (no subprocess), configurable with tool definitions, for testing MCP integration in unit tests without I/O
- **FR-692**: `synwire-test-utils` MUST provide `FakeUsage` enabling tests to inject exact `Usage` values (token counts, cost) without calling a real model. `FakeModel` MUST populate `Usage` from configured values
- **FR-693**: The `Vfs` conformance test suite MUST be a generic test harness parameterized over `impl Vfs`. It MUST test: all capability-advertised operations succeed, unsupported operations return `Unsupported`, path traversal is rejected, cwd persists across operations, concurrent access is safe
- **FR-694**: Proptest strategies MUST be provided for: `Directive` (all variants including Custom), `AgentEvent` (all variants), `GrepOptions` (all field combinations), `FsmTransition` (valid and invalid), `PermissionRule` (various glob patterns). These live in `synwire-test-utils`
- **FR-695**: Agent tests MUST be parallelizable — no shared mutable global state. `typetag` registry is append-only and process-global, which is safe for parallel tests. `FsmStrategy` instances are independent (each has its own `Mutex`). `MemoryProvider` instances are independent (each has its own `RwLock`)
- **FR-696**: `Schedule`, `Cron`, and `ApprovalTimeout` MUST accept an injectable clock trait (`trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; }`) rather than depending on wall-clock time. `synwire-test-utils` MUST provide `FakeClock` with controllable time for deterministic testing
- **FR-697**: SC-018 coverage target applies to line coverage (not branch coverage). Code behind `#[cfg(feature = "integration-tests")]` is excluded from the measurement since those tests require network access
- **FR-698**: Doc-test examples (`///` comments with code blocks) MUST compile and pass but are measured separately via `cargo make doctest`. They do NOT count toward the SC-018 coverage target

**Builder Validation & Defaults (FR-699–712)**:

- **FR-699**: `Agent::builder()` MUST require exactly two fields: `name` (String) and `model` (Box<dyn BaseChatModel>). All other fields are optional with documented defaults
- **FR-700**: Builder defaults: `max_turns` = 10, `max_budget` = None (unlimited), `permission_mode` = Default, `strategy` = DirectStrategy, `middleware` = empty stack, `hooks` = empty registry, `tools` = empty, `allowed_tools` = None (all), `excluded_tools` = empty, `effort` = None (model default), `thinking` = None (model default), `debug` = false, `sandbox` = None (disabled), `output_mode` = Tool, `system_prompt` = None (framework default), `env` = empty, `cwd` = None (process cwd)
- **FR-701**: `build()` MUST validate and return error for: (1) `allowed_tools` and `excluded_tools` both non-empty (use one or the other), (2) `output_schema` set but model lacks structured output capability, (3) `effort` set to a level not supported by the model, (4) `ThinkingConfig::Disabled` combined with `effort: Max` (contradiction)
- **FR-702**: `build()` MUST warn (log, not error) for: (1) `fallback_model` with fewer capabilities than primary, (2) `PermissionMode::BypassAll` in non-debug builds, (3) `max_budget` set without a model that reports cost
- **FR-703**: The default middleware stack when none is configured is empty — no middleware is implicitly added. To get filesystem tools, users MUST explicitly add `FilesystemMiddleware`
- **FR-704**: `OutputMode` automatic negotiation: if `output_schema` is set and model supports native structured output → `Native`; if model supports tool-calling → `Tool`; otherwise → `Prompt`. User-specified `output_mode` overrides auto-negotiation
- **FR-705**: `allowed_tools` and `excluded_tools` interact as follows: setting both is an error (FR-701). `allowed_tools = Some(list)` means only those tools are available. `excluded_tools = [list]` means all tools except those are available. Neither set means all tools available
- **FR-706**: When `fallback_model` has fewer capabilities (e.g., no tool-calling), the agent MUST downgrade `OutputMode` to `Prompt` when using the fallback, and restore the original mode when the primary model recovers
- **FR-707**: Invalid `PermissionRule` glob patterns MUST be detected at `build()` time with a clear error message including the invalid pattern
- **FR-708**: When `output_schema` is set but model doesn't support structured output, `build()` returns `AgentError::InvalidConfiguration("model X does not support structured output; remove output_schema or use a capable model")`
- **FR-709**: `InvalidTransition` error messages MUST include: current state, attempted action, and the list of valid actions from the current state (derived from the transition table)
- **FR-710**: `AgentError` display for `ModelError::RateLimit` MUST include retry-after duration (if available from provider headers) and utilization percentage
- **FR-711**: Hook timeout warnings MUST include: hook name, configured timeout, actual elapsed time, and the event that triggered the hook
- **FR-712**: `PermissionMode` presets MUST be documented with a decision matrix in rustdoc: when to use each mode, what it auto-approves, what it blocks, and common use cases (development, CI, production, sandboxed)

**VFS Abstraction (FR-713–726)** [IMPLEMENTED]:

- **FR-713**: System MUST provide a `Vfs` trait (replacing `Vfs`) defining a filesystem-like interface over heterogeneous data sources with operations mirroring Linux coreutils (ls, read, write, edit, grep, glob, find, tree, head, tail, stat, wc, du, diff, mkdir, touch, ln, chmod, cd, pwd, cp, mv, rm, upload, download)
- **FR-714**: System MUST provide `VfsCapabilities` bitflags (formerly `BackendCapabilities`) with flags for all VFS operations including PWD, CD, LS, TREE, READ, HEAD, TAIL, STAT, WC, DU, WRITE, APPEND, MKDIR, TOUCH, EDIT, DIFF, RM, CP, MV, LN, CHMOD, GREP, GLOB, FIND, UPLOAD, DOWNLOAD, EXEC, WATCH, INDEX, SEMANTIC_SEARCH
- **FR-715**: System MUST provide `VfsError` (replacing `BackendError`) with error variants including NotFound, PermissionDenied, PathTraversal, Unsupported, StaleRead, IndexNotReady, IndexDenied, Io
- **FR-716**: System MUST provide `MemoryProvider` (replacing `MemoryProvider`) for ephemeral in-memory file storage
- **FR-717**: System MUST provide `LocalProvider` (replacing `LocalProvider`) for real filesystem access with path traversal protection
- **FR-718**: System MUST provide `CompositeProvider` (replacing `CompositeProvider`) routing operations to sub-providers by mount path prefix
- **FR-719**: System MUST provide `StoreProvider` (replacing `StoreProvider`) for persistent cross-conversation storage
- **FR-720**: VFS MUST provide a `watch(path)` operation that records a file's current state (mtime, content hash) after every read, enabling stale-read detection
- **FR-721**: VFS MUST provide a `check_stale(path)` operation that returns `VfsError::StaleRead` if a file has been modified since the last `watch` call, preventing edits to externally-modified files
- **FR-722**: VFS tools MUST enforce a ReadGuard that tracks which files have been read in the current session, preventing edits to files that have not been read (avoids blind writes)
- **FR-723**: Sandbox concerns (Shell, ProcessManager, ArchiveManager, SandboxProtocol, approval gates) MUST be separated from VFS into a dedicated `sandbox` module in synwire-core
- **FR-724**: Git and HTTP backends MUST be removed from the VFS protocol — these are external service integrations, not filesystem operations
- **FR-725**: VFS tools MUST be generated from capabilities — only tools for operations the provider supports are included in the tool set returned by `vfs_tools()`
- **FR-726**: VFS output MUST support configurable formatting via `OutputFormat` (e.g., TOON compact format) for token-efficient LLM consumption

**AST-Aware Code Chunking (FR-727–732)** [IMPLEMENTED]:

- **FR-727**: System MUST provide a `synwire-chunker` crate with tree-sitter-based code chunking supporting at minimum: Rust, Python, JavaScript, TypeScript, Go, Java, C, C++, C#, Ruby, Bash, JSON, YAML, HTML, CSS
- **FR-728**: Chunker MUST extract top-level definition nodes (functions, structs, classes, enums, traits, impls, interfaces, type aliases) as semantic units, with each unit producing a separate `Document`
- **FR-729**: Each chunk `Document` MUST carry metadata: `file` (path), `language`, `line_start` (1-indexed), `line_end` (1-indexed), and `symbol` (definition name when extractable)
- **FR-730**: For languages without definition-level AST splitting (data/markup formats), chunker MUST return empty and fall back to recursive text splitting
- **FR-731**: Chunker MUST provide a `ChunkOptions` struct with configurable `chunk_size` and `overlap` for the text splitter fallback
- **FR-732**: Symbol name extraction MUST check child nodes for `identifier`, `name`, `field_identifier`, or `type_identifier` kinds

**Local Embeddings and Reranking (FR-733–735)** [IMPLEMENTED]:

- **FR-733**: System MUST provide a `synwire-embeddings-local` crate implementing the `Embeddings` trait via fastembed-rs. The default model MUST be `BAAI/bge-small-en-v1.5` (33MB, optimised for speed at Linux kernel scale). The model MUST be configurable to `bge-base-en-v1.5` (110MB) or `bge-large-en-v1.5` (335MB) via configuration or CLI flag (`--embedding-model`). Models are downloaded on first use and cached under `StorageLayout.models_cache()`
- **FR-734**: System MUST provide a `Reranker` implementation in `synwire-embeddings-local` using cross-encoder models for result reranking
- **FR-735**: Local embedding and reranking MUST operate without API keys or network access

**Vector Store (FR-736–737)** [IMPLEMENTED]:

- **FR-736**: System MUST provide a `synwire-vectorstore-lancedb` crate implementing the `VectorStore` trait with `add_documents` and `similarity_search_with_score` operations backed by LanceDB
- **FR-737**: Vector store MUST support per-index cache directories for isolated storage of different indexed paths

**Semantic Indexing Pipeline (FR-738–745)** [IMPLEMENTED]:

- **FR-738**: System MUST provide a `synwire-index` crate orchestrating: directory walking → AST-aware chunking → embedding → vector storage as a `SemanticIndex` struct. The pipeline MUST be streaming (process files one at a time, not load all into memory) to support repositories with 70,000+ files
- **FR-739**: Indexing MUST run asynchronously in a background tokio task, returning an `IndexHandle` immediately for status polling
- **FR-740**: Indexing MUST emit `IndexEvent`s (Progress, Complete, Failed, FileChanged) via an optional channel for async notification
- **FR-741**: Indexing MUST track content hashes (xxh128) per file and skip files whose content has not changed since the last index, enabling incremental re-indexing
- **FR-742**: Indexing MUST include a file watcher that detects changes after initial indexing and re-indexes affected files automatically
- **FR-743**: Indexing MUST persist cache metadata (path, timestamp, file count, chunk count, version) alongside the vector store for freshness checks
- **FR-744**: Semantic search MUST support optional cross-encoder reranking of vector similarity results via the `rerank` option
- **FR-744a**: The indexing pipeline MUST process files in streaming fashion: walk → chunk → embed → store one file (or small batch) at a time. The full file list MUST NOT be collected into memory before processing begins. Embedding calls MUST be batched (default: 32 chunks per batch) to balance throughput vs memory
- **FR-744b**: The content hash registry (`hashes.json`) MUST be loaded lazily or stored in SQLite for repos with 70K+ files. A 70K-entry JSON file (~3MB) is acceptable; if registry exceeds 10MB, migrate to SQLite
- **FR-744c**: Indexing progress MUST report: files processed / total files, chunks produced, elapsed time, estimated time remaining
- **FR-744d**: Indexing MUST support interruption and resumption: if the daemon is stopped mid-index, the next indexing run resumes from where it left off via the content hash registry (files already indexed are skipped)
- **FR-744e**: Files exceeding `max_file_size` (default: 1MB, configurable via `IndexOptions`) are skipped. Binary files (detected by null byte in first 8KB) are skipped. Both emit debug-level log entries
- **FR-744f**: The file watcher for 70K+ file trees MUST use recursive watching (single inotify watch on the root) rather than per-file watches to stay within OS limits. On Linux, `inotify` recursive watch via `notify` crate. If inotify watch limit is hit, fall back to periodic polling (configurable interval, default: 30 seconds) with a warning
- **FR-744g**: When search is attempted during active indexing, the search MUST return results from the currently-available (partial) index, not block until indexing completes. Results may be incomplete — the response MUST include a flag `index_in_progress: true` so the agent knows results are partial
- **FR-744h**: CPU utilisation during indexing SHOULD be bounded by a configurable parallelism limit (default: half of available cores) to avoid starving the editor and other development tools. Configurable via `metadata.indexing_parallelism` or daemon flag
- **FR-745**: Indexing the root filesystem (`/`) MUST be denied with `VfsError::IndexDenied` to prevent accidental full-disk indexing

**LSP Client Integration (FR-746–752)** [IMPLEMENTED]:

- **FR-746**: System MUST provide a `synwire-lsp` crate with an `LspClient` managing language server lifecycle (start, stop, reconnect), document synchronisation, and diagnostics caching
- **FR-747**: LSP tools MUST be generated conditionally based on the server's advertised capabilities — tools for unsupported operations are not included
- **FR-748**: LSP tools MUST include at minimum: `lsp_status`, `lsp_diagnostics`, `lsp_goto_definition`, `lsp_find_references`, `lsp_hover`, `lsp_completion`, `lsp_document_symbols`, `lsp_workspace_symbols`, `lsp_code_actions`, `lsp_formatting`, `lsp_rename`, `lsp_signature_help`
- **FR-749**: System MUST provide an LSP plugin for agent lifecycle integration (start server on agent start, stop on agent stop)
- **FR-750**: System MUST provide an LSP registry for managing multiple language servers, routing requests to the appropriate server by file language
- **FR-751**: `LspClient` MUST cache diagnostics received via `textDocument/publishDiagnostics` notifications, queryable per-file without additional server requests
- **FR-752**: `LspClient` MUST synchronise document state with the server via `textDocument/didOpen`, `textDocument/didChange`, and `textDocument/didClose` notifications

**DAP Client Integration (FR-753–757)** [IMPLEMENTED]:

- **FR-753**: System MUST provide a `synwire-dap` crate with a `DapClient` managing debug sessions via the Debug Adapter Protocol
- **FR-754**: DAP tools MUST include at minimum: breakpoint management, step (in/over/out), continue, pause, evaluate expression, disassemble, context inspection, variable setting, session info
- **FR-755**: System MUST provide a DAP plugin for agent lifecycle integration
- **FR-756**: System MUST provide a DAP registry for managing multiple debug adapters
- **FR-757**: DAP transport MUST support stdio (subprocess) communication with JSON wire protocol codec

**Process Sandboxing (FR-758–761)** [IMPLEMENTED]:

- **FR-758**: System MUST provide a `synwire-sandbox` crate with process isolation, output capture, and visibility controls
- **FR-759**: Sandbox MUST provide a process registry tracking all spawned processes
- **FR-760**: Sandbox MUST support platform-specific isolation (Linux namespaces)
- **FR-761**: Sandbox MUST provide a plugin interface for lifecycle management (start/stop/configure sandbox)

**Per-Method AST Chunking (FR-762–765)** [DRAFT]:

- **FR-762**: Chunker MUST recurse one level into container nodes (`impl_item` in Rust, `class_body`/`class_declaration` in JS/TS/Java/Python/C#/Ruby) to produce per-method chunks rather than one chunk per container
- **FR-763**: Per-method chunks MUST include the parent type name as a context prefix in the `symbol` metadata field (e.g., `Foo::bar` for method `bar` in `impl Foo`)
- **FR-764**: Top-level functions not inside a container MUST continue to produce chunks as before (no behavioural change for non-container definitions)
- **FR-765**: The chunker MUST NOT recurse deeper than one level — nested helper functions/closures inside methods remain part of their containing method's chunk

**File Skeleton Generation (FR-766–769)** [DRAFT]:

- **FR-766**: VFS MUST provide a `skeleton(path)` operation that returns a structural summary of a source file containing only definition signatures (function signatures, struct/class declarations, type aliases) without bodies
- **FR-767**: Skeleton generation MUST use tree-sitter to identify definition nodes and emit only the first line / declaration line of each, stripping body content
- **FR-768**: Skeleton output MUST include line numbers alongside signatures to enable subsequent `read_range` of specific definitions
- **FR-769**: For files in unsupported languages, `skeleton` MUST fall back to returning the full file content (graceful degradation)

**Hierarchical Narrowing (FR-770–773)** [DRAFT]:

- **FR-770**: System MUST provide a hierarchical narrowing localization tool or middleware implementing a three-phase pipeline: (1) directory tree analysis, (2) file skeleton / symbol analysis, (3) targeted line-range reading
- **FR-771**: Phase 1 MUST use the VFS `tree` operation to provide repository structure to the LLM for file-level ranking
- **FR-772**: Phase 2 MUST use `skeleton` or LSP `document_symbols` to provide function-level structure to the LLM for function-level ranking
- **FR-773**: Phase 3 MUST use `read_range` to provide only the relevant code sections to the LLM, minimising context window usage

**Code Dependency Graph (FR-774–779)** [DRAFT]:

- **FR-774**: System MUST provide a code graph construction capability (feature-gated as `code-graph` on `synwire-index`) that extracts definition→reference edges, import relationships, and call edges from tree-sitter ASTs
- **FR-775**: Code graph MUST store nodes (files, classes, functions) and typed edges (calls, imports, contains, inherits) as a lightweight adjacency structure alongside the vector index
- **FR-776**: System MUST provide a `graph_query(symbol, depth, direction)` VFS operation for traversing the code graph by symbol name, configurable hop depth, and direction (callers/callees/both)
- **FR-777**: System MUST provide a `graph_search(query, hops)` VFS operation combining embedding similarity search with ego-graph expansion — find nearest chunk by embedding, expand N hops in the code graph, return the subgraph
- **FR-778**: Code graph construction MUST be incremental — when a file changes, only its edges are recomputed, not the entire graph
- **FR-779**: Code graph MUST handle cross-file references by matching symbol names from tree-sitter `identifier` nodes against the definition index
- **FR-779a**: Code graph storage MUST be disk-backed via SQLite in WAL mode to support repositories with 70,000+ files and millions of edges. SQLite WAL provides concurrent reads during graph construction (search while indexing) without external locking

**Hybrid BM25 + Vector Search (FR-780–783)** [DRAFT]:

- **FR-780**: System MUST provide a BM25/TF-IDF index alongside vector embeddings in `synwire-index`, built during the same indexing pipeline. The BM25 index MUST be disk-backed (e.g., tantivy) to support repositories with 70,000+ files without excessive memory usage
- **FR-781**: System MUST provide a `hybrid_search(query, alpha, top_k)` VFS operation combining BM25 and vector scores with configurable alpha weighting (0.0 = pure vector, 1.0 = pure BM25)
- **FR-782**: BM25 index MUST support the same file filter, min_score, and rerank options as semantic search
- **FR-783**: BM25 index MUST be updated incrementally alongside the vector index when files change

**Test-Guided Fault Localization (FR-784–787)** [DRAFT]:

- **FR-784**: System MUST provide an SBFL module that computes Ochiai suspiciousness scores from test coverage data, ranking functions by likelihood of containing the fault
- **FR-785**: Coverage data MUST be obtainable from the DAP client (`synwire-dap`) when the debug adapter supports coverage reporting
- **FR-786**: System MUST provide a tool or middleware that combines SBFL scores with semantic search results, allowing LLM reranking of the fused results
- **FR-787**: When no coverage data is available, the system MUST gracefully fall back to standard semantic search without SBFL scores

**Repository Memory (FR-788–791)** [DRAFT]:

- **FR-788**: System MUST provide a persistent experience pool storing: edit→file associations (which files were modified for which issues), file summaries (LLM-generated descriptions of frequently edited files), and issue→resolution mappings
- **FR-789**: Experience pool MUST be queryable by concept (semantic search over summaries), by file path (direct lookup), and by issue keywords (keyword search over associations)
- **FR-790**: Experience pool MUST be updated automatically when the agent completes edits: recording the files touched, the issue description, and an LLM-generated summary of the change
- **FR-791**: Experience pool MUST persist across sessions, stored alongside or within the checkpoint storage

**Dynamic Call Graph (FR-792–794)** [DRAFT]:

- **FR-792**: System MUST support incremental call graph construction during agent search by following LSP go-to-definition edges on demand rather than pre-computing a full static graph
- **FR-793**: Dynamic graph construction MUST detect and handle cycles (visiting an already-known node) by stopping traversal on that path
- **FR-794**: When no LSP server is available, dynamic graph construction MUST fall back to static graph (if available) or grep-based symbol navigation

**MCTS-Based Search (FR-795–798)** [DRAFT]:

- **FR-795**: System MUST provide an MCTS execution strategy or search tool that explores multiple localization/repair trajectories in parallel using Monte Carlo Tree Search
- **FR-796**: MCTS MUST use a configurable value function (numerical + qualitative feedback) to score exploration paths
- **FR-797**: MCTS MUST support configurable search depth and compute budget, with performance scaling as budget increases
- **FR-798**: MCTS MUST use a discriminator or critic for patch/trajectory selection when multiple candidates are available

**Self-Evolving Tool Creation and Agent Skills (FR-799–815a)** [DRAFT]:

- **FR-799**: System MUST support a `CreateTool` directive variant (or equivalent plugin mechanism) enabling agents to register new tools during a session
- **FR-800**: Dynamically-created tools MUST pass through the same sandbox, permission, and approval checks as native tools
- **FR-801**: Tool definitions created by agents MUST include name, description, parameter schema, and implementation — where implementation is one of: (a) a sequence of existing tool invocations, (b) a Lua script, (c) a Rhai script, (d) an Extism WASM module reference, or (e) an external script (subprocess invocation). External scripts are permitted but discouraged — they bypass the embedded runtime sandboxing and are subject to the agent's `SandboxConfig` command restrictions instead
- **FR-802**: Dynamically-created tools MAY be persisted as agent-skill manifests in the experience pool and skills directory for use in future sessions
- **FR-803a**: System MUST provide a `synwire-agent-skills` crate implementing the [Agent Skills specification](https://agentskills.io/specification): skill discovery, validation (using the `skills-ref` validation rules), loading with progressive disclosure, and execution with runtime bindings
- **FR-803b**: Skills MUST follow the Agent Skills directory convention: a directory containing a `SKILL.md` file with YAML frontmatter and Markdown instructions, plus optional `scripts/`, `references/`, and `assets/` subdirectories
- **FR-803c**: `SKILL.md` frontmatter MUST support the standard fields: `name` (required, 1-64 chars, lowercase alphanumeric + hyphens, must match directory name), `description` (required, 1-1024 chars), `license` (optional), `compatibility` (optional, 1-500 chars), `metadata` (optional, string→string map for author/version/etc.), `allowed-tools` (optional, experimental, space-delimited tool list)
- **FR-803d**: Synwire MUST extend the standard frontmatter with an optional `runtime` field (one of `lua`, `rhai`, `wasm`, `tool-sequence`, `external`) specifying the execution backend for scripts in the `scripts/` directory. When `runtime` is absent, scripts are executed as external subprocesses per the standard Agent Skills model
- **FR-803e**: System MUST support progressive disclosure as defined by the spec: (1) at startup, load only `name` and `description` (~100 tokens per skill) for discovery, (2) when a task matches, load the full `SKILL.md` body (<5000 tokens recommended), (3) load `scripts/`, `references/`, `assets/` files only when referenced during execution
- **FR-803f**: System MUST support Lua as an embedded scripting runtime for skills via `mlua` (LuaJIT or Lua 5.4), with VFS operations exposed as Lua functions (e.g., `vfs.grep(pattern, opts)`, `vfs.read(path)`)
- **FR-803g**: System MUST support Rhai as an embedded scripting runtime for skills, with VFS operations exposed as Rhai functions. Rhai's sandboxed-by-default design (no filesystem/network access unless explicitly granted) makes it suitable for untrusted skill code
- **FR-803h**: System MUST support Extism WASM as a plugin runtime for skills, enabling portable, sandboxed skill execution. WASM skills communicate with the host via Extism PDK host functions that map to VFS operations. WASM skills have no implicit host access — only capabilities declared in `allowed-tools` are granted. WASM skill directories MUST preserve the original source code alongside the compiled `.wasm` module (e.g., `scripts/plugin.rs` + `scripts/plugin.wasm`), enabling auditability, recompilation, and modification by agents or developers
- **FR-803i**: System MUST provide a global skills directory (`$DATA/<product>/skills/`) via `StorageLayout.skills_dir()`, plus project-local skills in `.<product>/skills/` at the repository root (e.g., `.claude-code/skills/`, `.acme-agent/skills/`). The project-local directory name is derived from the `StorageLayout` product name and is configurable via `StorageLayout.project_skills_dirname()`. Skills in both locations are auto-discovered on startup. Project-local skills take precedence over global skills with the same name
- **FR-803j**: The skill loader MUST validate `SKILL.md` frontmatter against the Agent Skills spec constraints (name format, description length, directory name match), check that the declared runtime (if any) is available, and verify referenced scripts/files exist before registering the skill
- **FR-803k**: Skills MUST be versionable via the `metadata.version` field — when a skill with the same name but a higher version is loaded, it replaces the previous version. Downgrade is not automatic
- **FR-803l**: External script execution (no `runtime` field, or `runtime: external`) is permitted but discouraged
- **FR-803m**: `SKILL.md` body SHOULD be under 500 lines. Detailed reference material SHOULD be split into `references/` files loaded on demand. File references MUST use relative paths from the skill root, kept one level deep
- **FR-803n**: VFS host functions exposed to Lua/Rhai MUST include at minimum: `vfs.read(path)`, `vfs.grep(pattern, opts)`, `vfs.glob(pattern)`, `vfs.tree(path, opts)`, `vfs.head(path, opts)`, `vfs.stat(path)`. Write operations (`vfs.write`, `vfs.edit`) are only available if the skill's `allowed-tools` includes them
- **FR-803o**: Lua instruction count limit default: 1,000,000 instructions. Rhai max_operations default: 1,000,000. Both configurable per-skill via `metadata.max_operations` in frontmatter
- **FR-803p**: WASM skills that attempt to call host functions not listed in their `allowed-tools` MUST receive a permission denied error, not a crash or panic
- **FR-803q**: Skill error results MUST be returned to the LLM as tool errors (same format as native tool errors via `ToolOutput` with `status: Failure`), including the skill name and error message
- **FR-803r**: Tool-sequence runtime: implementation is a JSON array of tool invocation objects in the `scripts/` directory, e.g., `scripts/sequence.json`: `[{"tool": "grep", "args": {"pattern": "TODO"}}, {"tool": "skeleton", "args": {"path": "$result[0].file"}}]`. Semantics: `$result[N]` is the JSON output of step N (0-indexed). `$result[N].field` accesses a field. `$result[N][M]` indexes into arrays. On step failure, the sequence aborts and returns the error from the failing step. No implicit retry
- **FR-803s**: Skills are discovered in filesystem order within each directory. No explicit priority mechanism beyond project-local > global precedence. Deterministic ordering per platform
- **FR-803t**: When `metadata.version` is absent, the skill is treated as version "0.0.0" for precedence comparisons. Version comparison uses semantic versioning (semver) — major.minor.patch
- **FR-803u**: Rhai scripts MUST NOT be able to `import` external modules — only host-provided VFS functions are available. Lua scripts MUST have `require` disabled (sandbox mode) — only host-provided modules are available
- **FR-803v**: WASM skill memory allocation is bounded by Extism's default memory limit (configurable per-skill via `metadata.max_memory_mb`, default: 64MB). Exceeding the limit terminates the plugin with an OOM error
- **FR-803w**: Hot-reloading of skills during a session is NOT supported. Skills are loaded at MCP server startup. To add/modify skills, restart the MCP server (FR-888i)
- **FR-803x**: The `CreateTool` directive auto-generates a `SKILL.md` file with: `name` derived from the tool name, `description` from the directive's description field, `runtime` from the script language, the script written to `scripts/`. The generated skill is written to `$DATA/<product>/skills/` for persistence — the skill loader MUST emit a warning recommending embedded runtimes (lua, rhai, wasm) for better sandboxing

**GraphRAG Community Detection (FR-806–815)** [DRAFT]:

- **FR-806**: System MUST provide a `community-detection` feature flag on `synwire-index` that integrates `hit-leiden` for hierarchical community detection over the code dependency graph
- **FR-807**: Community detection MUST operate on the code graph from FR-774 (nodes = code entities, edges = calls/imports/contains/inherits) using the Leiden algorithm with configurable resolution parameter
- **FR-808**: Community detection MUST produce a hierarchical partition with multiple levels — coarse communities (module-level groupings) at higher levels and fine communities (function-level groupings) at lower levels
- **FR-809**: System MUST provide incremental community updates via `CommunityState::update()` when files change — delta edges computed from AST changes are applied without full reclustering
- **FR-810**: `CommunityState` MUST be serialisable via `into_parts()`/`from_parts()` and persistable in the synwire checkpoint system alongside the vector index and code graph
- **FR-811**: System MUST provide a `communities(path, level)` VFS operation listing communities at a given hierarchy level for an indexed path
- **FR-812**: System MUST provide a `community_members(community_id)` VFS operation returning the symbols and files belonging to a community
- **FR-813**: System MUST provide a `community_summary(community_id)` operation that returns (or generates on demand) an LLM-generated summary describing the community's purpose, key members, and relationships
- **FR-814**: System MUST provide a `community_search(query, level)` VFS operation that searches community summaries at a given hierarchy level, enabling multi-resolution retrieval — coarse search first, then drill into members
- **FR-815**: When `IndexEvent::FileChanged` fires, the community detection pipeline MUST compute delta edges from the changed file's updated AST, call `CommunityState::update()`, and mark affected community summaries as stale for regeneration

**Persistent Storage Layout (FR-816–832)** [DRAFT]:

- **FR-816**: System MUST provide a `StorageLayout` struct parameterised by `product_name: String` that computes all persistent storage paths for the product, using platform conventions (XDG on Linux, `~/Library` on macOS, `%LOCALAPPDATA%` on Windows) via the `directories` crate
- **FR-817**: `StorageLayout` MUST separate durable data (`$XDG_DATA_HOME/<product>/`) from rebuildable caches (`$XDG_CACHE_HOME/<product>/`). Durable data includes session checkpoints, community summaries, experience pool, and code graph. Cache data includes vector indices, BM25 indices, content hashes, LSP/DAP caches, and downloaded models
- **FR-818**: `StorageLayout` MUST support a `root_override: Option<PathBuf>` that places all storage (durable + cache) under a single directory, for use in CI, Docker, and portable installations
- **FR-819**: System MUST provide two-level project identity: `RepoId` (Git first-commit hash, identifying the repository family across all worktrees) and `WorktreeId` (`RepoId` + sha256 of the worktree root path, identifying a specific working copy). For non-Git directories, `RepoId` falls back to `sha256(canonical_path)` and `WorktreeId` equals `RepoId`. Two Git worktrees of the same repository share a `RepoId` but have distinct `WorktreeId`s
- **FR-819a**: The index coordinator (FR-825a) operates per-`RepoId` (shared embedding model, shared base graph metadata) but maintains per-`WorktreeId` indices (vector store, BM25, code graph, community state) since file contents differ between worktrees/branches
- **FR-820**: `WorktreeId` MUST include a human-readable `display_name` (derived from repository name + branch name or directory name) for logging and debugging
- **FR-821**: `StorageLayout` MUST provide typed path accessors for each subsystem: `session_db(session_id) -> PathBuf`, `index_cache(project_id) -> PathBuf`, `graph_dir(project_id) -> PathBuf`, `communities_dir(project_id) -> PathBuf`, `experience_db(project_id) -> PathBuf`, `lsp_cache(project_id) -> PathBuf`, `models_cache() -> PathBuf`
- **FR-822**: `StorageLayout` path accessors MUST create parent directories on first access (lazy creation) — directories are not created at `StorageLayout` construction time
- **FR-823**: The existing `synwire-checkpoint` and `synwire-checkpoint-sqlite` crates MUST NOT be modified — they are already path-agnostic. `StorageLayout` provides paths that are passed to `SqliteSaver::new(path)` and `IndexConfig.cache_base`
- **FR-824**: `StorageLayout` MUST replace the hardcoded `"synwire"` product name in `synwire-index/src/cache.rs` — the index cache directory MUST be derived from `StorageLayout.index_cache(project_id)` instead of `$CACHE/synwire/indices/<sha256>/`
- **FR-825**: System MUST use each storage backend's native concurrency model instead of external file locks for inter-process coordination: SQLite in WAL mode (concurrent readers + single writer, readers never blocked) for structured data (checkpoints, experience pool, registry, dependency index, xrefs, code graph edges), LanceDB's native concurrent read support for vector indices, and tantivy's built-in IndexWriter/IndexReader model for BM25 indices. External file locking (`flock`/`LockFile`) is NOT used for these stores
- **FR-825a**: System MUST provide a `synwire-daemon` background process that runs as a singleton per product (one daemon for all repos, all worktrees, all cloned repos). The daemon owns: a single embedding model instance, all file watchers, all indexing pipelines, the global tier (registry, dependency index, xref graph, experience pool), and per-`WorktreeId` indices. MCP server instances connect to the daemon via a Unix domain socket (`$DATA/<product>/daemon.sock`) or named pipe on Windows
- **FR-825b**: The first MCP server instance MUST auto-start the daemon if it is not already running by spawning `synwire-daemon` as a detached child process (`setsid` + double-fork on Unix, `CREATE_NO_WINDOW` on Windows) — no systemd, launchctl, or service manager required. The daemon is a regular user process that self-manages its lifecycle. Subsequent MCP servers detect the running daemon via the socket file and connect as clients. If the daemon crashes, the next MCP server to need it restarts it (leader election via atomic PID file at `$DATA/<product>/daemon.pid`)
- **FR-825c**: The daemon MUST manage multiple repos and worktrees simultaneously as internal async tasks (not separate processes). Each `WorktreeId` has its own index (vector store, BM25, code graph, community state). The embedding model, tree-sitter parsers, and cross-encoder reranker are shared across all repos/worktrees
- **FR-825d**: MCP servers MUST be thin stdio↔UDS proxies: they receive MCP tool calls over stdio from the editor, forward index/search/graph/community requests to the daemon via UDS, and relay results back. VFS file operations (read, write, edit, grep) that don't require the index MAY be handled directly by the MCP server without routing through the daemon
- **FR-825e**: The daemon MUST handle `clone_repo` requests: clone the repository, register it with a new `RepoId`/`WorktreeId`, start indexing, and make it searchable — all within the single daemon process. No additional coordinator process is spawned
- **FR-825f**: The daemon MUST handle cross-project operations: building the global dependency index (parsing manifests from all registered repos), constructing the cross-project xref graph (linking symbols across locally-indexed repos), and serving cross-project queries (`xref_query`, global experience pool searches)
- **FR-825g**: When all MCP server clients disconnect, the daemon MUST remain running for a configurable grace period (default: 5 minutes, configurable via `--daemon-grace-period` CLI flag on `synwire-daemon` and `grace_period_seconds` in the config file) to handle rapid editor restarts without re-loading the embedding model or losing file watcher state. After the grace period with no clients, the daemon exits cleanly, releasing the socket file and PID file
- **FR-825h**: The daemon MUST be startable independently via `synwire-daemon` CLI (not only auto-started by MCP servers), enabling pre-warming: `synwire-daemon --product-name claude-code --project /path/to/repo` starts the daemon, loads the embedding model, and begins indexing before any editor is opened
- **FR-826**: For data stored as serialised binary blobs (e.g., `CommunityState` via `into_parts()`) that lack a native concurrent-access storage backend, writes MUST use atomic write-to-temp-then-rename to prevent partial reads by concurrent processes. No advisory locks are required — atomic rename guarantees readers see either the old or new version, never a partial write
- **FR-827**: System MUST provide a `StorageMigration` trait with `current_version() -> u32` and `migrate(from, to, path) -> Result<()>`. Each subsystem (index, graph, communities, experience, sessions) implements its own migrations
- **FR-828**: On first access, each subsystem MUST check a `version.json` file in its directory and run migrations if the stored version is older than the current version. Failed migrations MUST leave the previous data intact (copy-then-swap strategy)
- **FR-829**: Configuration hierarchy for storage root MUST be (highest to lowest): `SYNWIRE_DATA_DIR` environment variable → `StorageLayout::with_root()` programmatic override → project-local `.synwire/config.json` → platform default
- **FR-830**: Each project directory MUST contain a `project.json` file recording `ProjectId`, `display_name`, `canonical_path`, `remote_url` (if Git), and `created_at` timestamp, enabling re-association if paths change
- **FR-831**: `StorageLayout` MUST provide a `discover_project(path) -> Option<ProjectId>` method that searches for an existing project by computing its `ProjectId` and checking if data already exists, enabling transparent project relocation
- **FR-831a**: `discover_project(path)` MUST return the first matching `WorktreeId` by `RepoId`. If multiple data directories share the same `RepoId` (multiple worktrees), all `WorktreeId`s are returned and the caller selects by path match
- **FR-831b**: Orphaned project data (project deleted, directory moved, never accessed for >90 days configurable) MAY be cleaned up by a `storage_gc` operation that lists stale entries from the registry and removes their data directories after confirmation
- **FR-831c**: `version.json` format: `{"subsystem": "<name>", "version": <u32>, "migrated_at": "<rfc3339>"}`. One file per subsystem directory (index, graph, communities, experience)
- **FR-831d**: File permissions: SQLite database files and the daemon socket MUST be created with 0600 permissions on Unix. Log files MUST be 0640 (readable by group for log aggregation). Directory permissions MUST be 0700
- **FR-831e**: For monorepos where multiple "projects" live in subdirectories of the same Git repo, `WorktreeId` is per-worktree-root (the repo root), not per-subdirectory. Subdirectory projects share the same index. The agent uses file filters (`SemanticSearchOptions.file_filter`) to scope searches to a subdirectory
- **FR-831f**: For repos with multiple Git remotes, `project.json` records the `origin` remote URL. If no `origin` exists, the first remote is used. If no remotes exist, `remote_url` is null
- **FR-831g**: For repos with multiple root commits (octopus merges, grafted history), `git rev-list --max-parents=0 HEAD` may return multiple hashes. `RepoId` MUST use the first (oldest) hash, sorted lexicographically for determinism
- **FR-832**: `BaseStore` (from `synwire-checkpoint`) MUST be usable as the experience pool backend — the namespace-isolated key-value store with search capability is a natural fit for edit associations and file summaries. `StorageLayout` provides the database path; `BaseStore` provides the access pattern
- **FR-833**: `StorageLayout` MUST provide a `global/` storage tier alongside per-project storage, for data that spans multiple projects. Global path accessors: `global_experience_db() -> PathBuf`, `global_dependency_db() -> PathBuf`, `global_registry() -> PathBuf`, `global_config() -> PathBuf`
- **FR-834**: System MUST provide a project registry (`global/registry.json`) tracking all known projects with metadata: `ProjectId`, `display_name`, `canonical_path`, `remote_url`, `last_accessed`, `tags`. The registry is updated on every project access
- **FR-835**: System MUST provide a global dependency index (`global/dependencies/deps.db`) storing project→dependency edges (e.g., project A depends on `sigs.k8s.io/cluster-api` v1.9.0). Queryable by dependency name ("which of my projects use library X?") and by project ("what does project Y depend on?")
- **FR-836**: The dependency index MUST be populated from project manifest files (`Cargo.toml`, `go.mod`, `package.json`, `pyproject.toml`) during indexing, with the parser extensible per language ecosystem
- **FR-837**: The experience pool MUST operate in two tiers: project-local (`projects/<pid>/experience/`) for project-specific edit associations, and global (`global/experience/`) for cross-project patterns and fix recipes. Queries MUST search project-local first, then fall back to global
- **FR-838**: When an agent records an edit association, it MUST be written to the project-local experience pool. Optionally, generalised patterns (e.g., "auth middleware timeout fix") MAY be promoted to the global pool for cross-project reuse
- **FR-839**: Global stores MUST use the same `BaseStore` trait as per-project stores backed by SQLite in WAL mode — no new storage abstraction required. Concurrent access from multiple MCP server processes is handled by SQLite's native WAL concurrency (no external lock files)

**Standalone MCP Server Binary (FR-858–872)** [DRAFT]:

- **FR-858**: System MUST provide a `synwire-mcp-server` binary crate that exposes synwire tools as an MCP server, runnable as a standalone process
- **FR-859**: The binary MUST use stdio transport exclusively (JSON-RPC over stdin/stdout). Each code editor instance (Claude Code window, Copilot instance, Cursor tab) spawns its own MCP server process. No HTTP transport is provided — process-level isolation via the OS is the security model
- **FR-860**: The binary MUST accept `--product-name <name>` (default: `"synwire"`) to configure the `StorageLayout` product namespace, enabling isolation between different deployments
- **FR-861**: The binary MUST accept `--project <path>` to set the initial project root. If omitted, the server starts without a project and the client uses the `index` tool to specify one
- **FR-862**: The MCP server MUST expose the following VFS tools as MCP tool definitions: `read`, `write`, `edit`, `grep`, `glob`, `find`, `tree`, `head`, `tail`, `stat`, `ls`, `diff`, `semantic_search`, `hybrid_search`, `skeleton`, `index`, `index_status`
- **FR-863**: The MCP server MUST expose code graph tools: `graph_query`, `graph_search`, `community_search`, `community_members`
- **FR-864**: The MCP server MUST expose the `clone_repo` tool for on-demand repository cloning and mounting
- **FR-865**: The MCP server MUST optionally expose LSP tools (`lsp_goto_definition`, `lsp_find_references`, `lsp_hover`, `lsp_document_symbols`, `lsp_workspace_symbols`, `lsp_diagnostics`, `lsp_rename`) when configured with `--lsp <command>` specifying the language server to start
- **FR-866**: The MCP server MUST optionally expose DAP tools when configured with `--dap <command>` specifying the debug adapter to start
- **FR-867**: Tool definitions MUST include complete JSON Schema parameter descriptions following MCP tool specification, with descriptions written for LLM consumption (explaining when to use each tool)
- **FR-868**: The MCP server MUST handle concurrent tool calls from the client, using `ProjectLock` shared locks for read operations and exclusive locks for write operations
- **FR-869**: The MCP server MUST emit MCP notifications for long-running operations (indexing progress, clone progress) so clients can display status
- **FR-870**: The binary MUST be distributable as a single static binary (via `cargo install synwire-mcp-server` or prebuilt release binaries) with no external runtime dependencies beyond Git (for `clone_repo`) and optionally a language server binary (for LSP tools)
- **FR-871**: The MCP server MUST support a `--config <path>` flag for a TOML/JSON configuration file as an alternative to CLI flags, covering all options (product name, project path, LSP command, DAP command, log level, embedding model)
- **FR-872**: Multiple MCP server instances MUST safely share the same persistent data (indices, graphs, communities, experience pool) via each backend's native concurrency: SQLite WAL for structured data, LanceDB concurrent reads for vectors, tantivy IndexReader snapshots for BM25, atomic rename for binary blobs. No external file locks are required. Each instance is a separate OS process spawned by its respective editor
- **FR-873**: When one MCP server instance completes indexing, other instances targeting the same project MUST see the updated index on their next query — SQLite WAL makes new rows visible to readers immediately after commit, LanceDB readers see the latest manifest on next query, tantivy IndexReaders refresh to the latest commit point
- **FR-874**: The MCP server process MUST exit cleanly when the client (editor) closes the stdio pipe — background indexing and file watchers are cancelled on process exit since the process is scoped to the editor session
- **FR-875**: The MCP server MUST emit structured logs via the `tracing` crate to stderr at info level by default, configurable via `RUST_LOG` environment variable or `--log-level` CLI flag. Stderr MUST NOT interfere with the MCP JSON-RPC protocol on stdout
- **FR-876**: The MCP server MUST additionally write logs to rotated log files under `StorageLayout` (`$DATA/<product>/logs/`), enabling post-mortem analysis after the process has exited. Log rotation MUST be daily with a configurable retention period (default: 7 days)
- **FR-877**: `StorageLayout` MUST provide a `logs_dir() -> PathBuf` accessor for the log directory
- **FR-878**: Log entries for indexing operations MUST include: files processed, chunks produced, duration, and any errors/skips. Log entries for search operations MUST include: query, result count, and latency

**MCP Sampling for Tool-Internal LLM Access (FR-879–884)** [DRAFT]:

- **FR-879**: The MCP server MUST support MCP sampling — requesting the client to make an LLM call on the server's behalf — for features that require LLM reasoning within a tool invocation (community summary generation, hierarchical narrowing file/function ranking, experience pool summary generation, SBFL reranking, MCTS value function scoring)
- **FR-880**: System MUST provide a `SamplingProvider` trait abstracting LLM access for tool-internal use, with two implementations: (a) MCP sampling (delegates to the client via `sampling/createMessage`), (b) direct model invocation (uses a configured `BaseChatModel` for standalone/non-MCP use)
- **FR-881**: All sampling calls MUST be lazy/on-demand — never triggered during batch indexing, graph construction, or community detection. Summaries and rankings are generated only when explicitly requested by a tool invocation (e.g., `community_summary(id)` triggers one sampling call for that community, not for all communities). Indexing 70,000 files MUST produce zero sampling calls
- **FR-882**: Community summary generation (FR-813) MUST use `SamplingProvider` on first access to `community_summary(id)`. The result is cached. Subsequent calls return the cached summary until the community is invalidated by a member change. There is no batch "summarise all communities" operation
- **FR-883**: Hierarchical narrowing phases 1 and 2 (FR-771–772) MUST use `SamplingProvider` to rank candidate files and functions only when the narrowing tool is invoked by the agent. This is 2 sampling calls per narrowing invocation (one for file ranking, one for function ranking) — bounded and predictable
- **FR-884**: Experience pool summary generation (FR-790) MUST use `SamplingProvider` only when the agent completes an edit and the summary is explicitly requested or auto-generated post-edit. This is at most 1 sampling call per edit operation
- **FR-885**: When `SamplingProvider` is unavailable (MCP client doesn't support sampling, or no model configured in standalone mode), features requiring LLM access MUST degrade gracefully: community summaries return member lists instead of natural language summaries, hierarchical narrowing returns alphabetically-sorted candidates instead of ranked, experience pool stores raw edit associations without summaries
- **FR-886**: The total number of sampling calls per tool invocation MUST be bounded and documented: `community_summary` = 1, hierarchical narrowing = 2, experience pool summary = 1, SBFL reranking = 1, MCTS value scoring = 1 per trajectory. No tool may trigger unbounded sampling calls
- **FR-887**: All tool results returned to the LLM MUST use one of four formats: plain text, Markdown, TOON (Token-Oriented Object Notation), or JSON. No other serialisation formats (XML, YAML, HTML, binary, custom formats) are permitted in tool output. The format is selected via `OutputFormat` (FR-726) and MUST be consistent across all tools within a session
- **FR-888a**: The MCP server MUST comply with MCP protocol specification version 2025-06-18 (or latest stable). The server MUST report its protocol version in the `initialize` response
- **FR-888b**: MCP tool descriptions MUST follow a consistent format: first sentence states what the tool does, second sentence states when to use it, followed by parameter descriptions. Maximum 500 characters per tool description. Descriptions are optimised for LLM tool selection, not human reading
- **FR-888c**: The TOML/JSON config file schema MUST mirror all CLI flags: `product_name`, `project`, `lsp_command`, `dap_command`, `embedding_model`, `log_level`, `skills_dirs` (additional skill directories). CLI flags take precedence over config file values
- **FR-888d**: MCP progress notifications MUST use the `notifications/progress` method with fields: `progressToken` (operation ID), `progress` (0.0–1.0), `total` (optional), `message` (human-readable status). Emitted for: indexing, cloning, graph building, community detection
- **FR-888e**: MCP tool results for large outputs MUST be truncated at a configurable limit (default: 100KB) with a `[truncated at {limit}, {total} total]` suffix. No pagination — the agent should refine its query
- **FR-888f**: "Single static binary" means no runtime dependencies beyond Git (for `clone_repo`) and the OS. The binary MAY be dynamically linked against system libc. Statically linked (musl) builds are a distribution option, not a requirement
- **FR-888g**: Clean shutdown on stdio pipe close MUST complete within 5 seconds: cancel in-flight daemon requests, flush log buffers, close UDS connection. In-flight indexing in the daemon is NOT cancelled (the daemon continues independently)
- **FR-888h**: Log rotation retention (7 days default) MUST be configurable via `--log-retention-days` or config file
- **FR-888i**: Agent skills are discovered at MCP server startup only (not watched for changes during session). To load new skills, restart the MCP server. The daemon does not manage skills — skill loading is MCP-server-side
- **FR-888j**: `--embedding-model` with an unrecognised model name MUST fail at startup with a clear error listing available models, not at first search time
- **FR-888k**: Before `--project` is set or `index` is called, the MCP server MUST expose a minimal tool set: `index`, `clone_repo`, and VFS tools that don't require an index (read, write, edit, grep, glob, find, tree, ls, stat, head, tail). Search tools (semantic_search, hybrid_search, graph_query, community_search) return an error directing the user to call `index` first
- **FR-888l**: Tool call timeout: individual tool calls MUST have a configurable timeout (default: 120 seconds). On timeout, the MCP server returns an error result to the client. Daemon-side operations that have already started are NOT cancelled by a client-side timeout
- **FR-888m**: The MCP server startup sequence MUST be: (1) parse CLI/config, (2) initialise `StorageLayout`, (3) connect to or start daemon, (4) discover and load agent skills, (5) register tools, (6) begin accepting MCP requests. Startup MUST complete within 10 seconds excluding daemon start time
- **FR-888n**: `RUST_LOG` takes precedence over `--log-level` when both are set. `--log-level` sets the default; `RUST_LOG` overrides with per-module granularity
- **FR-888o**: MCP server reports capabilities to client in `initialize` response: `tools` (list of all registered tools with schemas), `serverInfo` (name: "synwire-mcp-server", version from Cargo.toml)
- **FR-888p**: When `--lsp` language server crashes, the daemon (or MCP server, for local LSP) MUST attempt automatic restart up to 3 times with exponential backoff. LSP tools return errors during restart. Same policy for `--dap`
- **FR-888q**: When `--project` points to a non-existent path, the MCP server MUST fail at startup with a clear error, not silently start without a project
- **FR-888r**: Malformed `SKILL.md` files in the skills directory are skipped with a warning log per file. The server starts with all valid skills loaded. One bad skill does not block others
- **FR-888s**: Concurrent tool calls from the same MCP client are supported — the MCP server forwards them to the daemon in parallel. Results are returned as they complete, not in request order
- **FR-888t**: When `StorageLayout` directories cannot be created (permissions), the MCP server MUST fail at startup with a clear error identifying which directory is not writable
- **FR-888u**: Sampling prompt templates are implementation details, not spec-level requirements. The spec defines WHAT is sent to sampling (community members for summary, file list for ranking) and the expected output format (natural language summary, ranked list), not the exact prompt wording
- **FR-888v**: Sampling timeout: 30 seconds per sampling call. On timeout, fall back to the no-sampling degradation path (FR-885). No retry — the agent can retry the tool call if desired
- **FR-888w**: When MCP client supports sampling but the model returns a refusal or content filter, treat as sampling unavailable for that call and use the degradation path (FR-885)
- **FR-888x**: Sampling prompts for large inputs (100+ community members, 50+ file skeletons) MUST be truncated to fit within 8192 tokens, including only the top-N most relevant items and a count of omitted items
**Tool Search and Progressive Discovery (FR-897–908)** [DRAFT]:

- **FR-897**: `synwire-core` MUST provide a `ToolSearchIndex` that enables progressive tool discovery at the framework level. Any agent (synwire runtime, third-party agents, MCP server) can use `ToolSearchIndex` to: (a) generate compact tool listings (name + description only, <2,000 tokens for 40+ tools), (b) retrieve full schemas for top-K relevant tools by query. This is a core library capability, not MCP-specific
- **FR-898**: `ToolSearchIndex` MUST provide a `tool_search(query, top_k)` method and a corresponding `StructuredTool` implementation (`tool_search` meta-tool) that any agent can include in its tool set. The meta-tool is always loaded with its full schema (it's the bootstrap tool)
- **FR-899**: `tool_search` MUST use hybrid retrieval combining: (a) vector similarity — embed the query and match against pre-computed tool + example-query embeddings using bge-small, (b) keyword boosting — tokenise the query and boost matches against tool fields with weights: namespace +5.0, name +3.0, description +2.0, tags +1.5. Normalised vector score and scaled text boost are fused and clamped to [0,1]. Additionally supports namespace browsing — `tool_search("namespace:graph")` returns all tools in a namespace without embedding search
- **FR-900**: Tools registered with `ToolSearchIndex` MUST declare a namespace. Standard namespaces: `file` (VFS read/write/edit), `search` (grep, glob, find, semantic_search, hybrid_search, skeleton), `graph` (graph_query, graph_search), `community` (communities, community_members, community_summary, community_search), `lsp` (all LSP tools), `dap` (all DAP tools), `skill` (dynamically loaded), `repo` (clone_repo, index, index_status). Tool names MUST be prefixed with namespace (e.g., `file.read`, `search.semantic`, `graph.query`)
- **FR-901**: Each tool registered with `ToolSearchIndex` MUST provide: `name`, `description` (max 100 chars, action verb + object + when-to-use hint), `namespace`, `tags` (keyword list for boosting), and 3-5 `example_queries` (representing "when would a user need this tool?"). Embeddings MUST be computed separately for the description AND each example query (multi-vector per tool), stored in the same LanceDB table with a `vector_type` discriminator. At search time, match against all vectors and take the max score per tool — this expands the search surface from ~40 to ~200-400 vectors, significantly improving recall for diverse query phrasings
- **FR-902**: `ToolSearchIndex` MUST track which tool schemas have been retrieved in the current session to support deduplication — repeated searches don't re-return already-loaded schemas
- **FR-903**: A `tool_list` meta-tool MUST be available that returns the full namespace-grouped tool listing with names and descriptions (no schemas) for the LLM to browse when unsure which namespace to search
- **FR-904**: The MCP server MUST use `ToolSearchIndex` to implement MCP-level progressive discovery: `tools/list` returns compact entries, `tool_search` meta-tool available as an MCP tool. For clients supporting `defer_loading` (Anthropic protocol extension), the server SHOULD use the native mechanism instead
- **FR-905**: The `ToolSearchIndex` embedding index MUST be rebuilt when tools change (skills loaded, LSP/DAP tools added). This is lightweight (~40 embeddings, <1 second)
- **FR-906**: The token budget for schemas returned by `tool_search` MUST be dynamically allocated with progressive disclosure depth: top-5 tools get full JSON Schema (~200-500 tokens each), next 10 get summary (name + description + parameter names, ~80 tokens each), remainder get name only (~5 tokens each). Total budget per `tool_search` call MUST be bounded at 5,000 tokens. If top-K tools exceed this, reduce disclosure depth rather than dropping tools
- **FR-906a**: `ToolSearchIndex` MUST support a `DisclosureDepth` enum: `Minimal` (name only, ~5 tokens), `Summary` (name + description, ~20 tokens), `Parameters` (name + description + param names/types, ~80 tokens), `Full` (complete JSON Schema, ~200-500 tokens). Each tool's `render(depth)` method produces output at the requested depth
- **FR-906b**: `ToolSearchIndex` MUST compute a deterministic hash of the sorted, serialised tool registry (using `BTreeMap` key ordering + sha256). Embeddings are only recomputed when the hash changes. This enables efficient incremental re-indexing when skills are added/removed
- **FR-907**: Tool search accuracy MUST be logged: which tools were searched for, returned, and actually invoked. Enables offline analysis and example query tuning
- **FR-908**: Third-party agents using synwire tools without `ToolSearchIndex` MUST still work — all tools function normally when their full schemas are loaded directly. `ToolSearchIndex` is an optimisation, not a gate
- **FR-909**: `ToolSearchIndex` MUST provide a `search_progressive(query, steps, per_step_k)` method implementing iterative residual retrieval (ProTIP pattern): (1) embed query, (2) retrieve top-per_step_k tools, (3) subtract retrieved tool embeddings from query embedding to reveal unfulfilled intent, (4) repeat from step 2 for `steps` iterations, (5) deduplicate and return combined results. This handles multi-step queries like "find the function definition, refactor it, and run the tests" where single-step retrieval returns only "find" tools. Evidence: 24% Recall@K=10 improvement, 41% plan accuracy improvement (ProTIP, Apple 2023)
- **FR-910**: `ToolSearchIndex` MUST maintain a `ToolTransitionGraph` recording tool co-invocation sequences from invocation logs (FR-907). Edge weights represent normalised transition probabilities with exponential decay (configurable half-life, default: 100 invocations — after 100 invocations, an edge's weight halves). During search, tools that commonly follow the most recently invoked tool receive a score boost (multiplicative, default 1.3x). The graph is persisted across sessions in SQLite and updated incrementally. Evidence: 30% inference cost reduction (AutoTool 2025), 40% fewer false positives (Less-is-More 2024)
- **FR-911**: `ToolSearchIndex` MAY perform optional query intent extraction before retrieval: strip conversational context and extract the core tool-related intent (verb-object pairs). A default `QueryPreprocessor` trait with pass-through implementation is provided; an `IntentExtractor` implementation uses heuristic rules (extract verb-object from queries exceeding N tokens). Evidence: 20-39% nDCG@5 improvement (Re-Invoke, EMNLP 2024)
- **FR-912**: `ToolSearchIndex` MUST apply seen/unseen adaptive scoring: tools already in `loaded_schemas` (FR-902) receive a score penalty (multiply by configurable factor, default 0.8) to bias results toward novel tools the model hasn't seen schemas for yet. This reduces wasted context on tools the model already knows. Evidence: +5 Recall@5 (ToolRerank, LREC-COLING 2024)
- **FR-913**: `ToolSearchResult` MUST include diagnostic metadata for low-confidence results (score below configurable threshold): `nearest_namespace` (closest namespace to query intent), `alternative_keywords` (related terms found in the index), `confidence_level` (high/medium/low). This enables the agent to reformulate queries without requiring `ToolSearchIndex` to do the reasoning
- **FR-914**: `ToolSearchIndex` MUST support a feedback loop from invocation logs: successful query→tool pairs (where the tool was invoked and succeeded) are extracted from FR-907 logs and added as additional example queries for the tool (capped at 10 most diverse per tool, diversity measured by embedding distance). Re-embedding runs as a background task during idle periods. Evidence: domain-specific training data substantially improves retrieval (ToolRet 2025)
- **FR-915**: `ToolSearchIndex` MAY apply post-retrieval parameter-type verification: compare the query's implied parameters (file paths suggest file tools, function names suggest LSP tools, regex patterns suggest search tools) against each candidate tool's parameter schema, demoting tools whose parameters don't match. This is heuristic-based (no model calls) and runs as a post-scoring filter before progressive disclosure assignment

- **FR-889**: All new crates (`synwire-storage`, `synwire-agent-skills`, `synwire-mcp-server`, `synwire-daemon`) MUST include a `README.md` with: crate purpose, quick start example, CLI usage (for binaries), and link to full documentation. Existing implemented crates (`synwire-chunker`, `synwire-index`, `synwire-lsp`, `synwire-dap`, `synwire-sandbox`, `synwire-embeddings-local`, `synwire-vectorstore-lancedb`) MUST have README.md added if missing
- **FR-890**: All new public types, traits, and functions in new crates MUST have rustdoc with: one-line summary, description, usage example (doc-tested where possible), and cross-references to related types. `cargo doc --no-deps` MUST produce zero warnings across the workspace
- **FR-891**: mdBook documentation (`docs/src/`) MUST be extended with: (a) explanation docs for StorageLayout architecture, code graph, community detection, hybrid search, agent skills architecture; (b) how-to guides for authoring agent skills, MCP server setup (Claude Code + Copilot), semantic search configuration; (c) tutorials for MCP server getting started, first agent skill, and semantic search workflow
- **FR-892**: Existing documentation MUST be updated: `feature-flags.md` (new feature flags), `crate-organisation.md` (new crates), architecture diagram (expanded crate graph), `glossary.md` (new terms: Vfs, StorageLayout, WorktreeId, RepoId, AgentSkill, SKILL.md, CodeGraph, CommunityState, HybridSearch, SamplingProvider, SynwireDaemon), `CLAUDE.md` (new crate paths and commands)
- **FR-893**: A standalone MCP server setup guide MUST be provided targeting end-users (not synwire contributors), covering: installation (`cargo install`), Claude Code config, Copilot config, CLI flags, config file, troubleshooting. Written as a mdBook how-to at `docs/src/how-to/mcp-server-setup.md`
- **FR-894**: A standalone `SKILL.md` authoring guide MUST be provided targeting skill authors, covering: SKILL.md format, frontmatter fields, directory structure, embedded runtimes (Lua/Rhai/WASM examples), testing skills locally, publishing. Written at `docs/src/how-to/authoring-skills.md`
- **FR-895**: `synwire-mcp-server --help` output MUST document all flags with descriptions and defaults. The help text is the primary reference for CLI users
- **FR-896**: A migration guide MUST be provided for users upgrading from the old hardcoded `$CACHE/synwire/indices/` paths to the new `StorageLayout` paths, covering: automatic migration behaviour, manual cleanup via `storage_gc`, and how to verify data was migrated correctly
- **FR-888**: Structured output from tools (tabular data, search results, graph query results, directory listings, diagnostics) MUST be serialised as either TOON or JSON. Plain text and Markdown are reserved for narrative/prose content (summaries, explanations, documentation). The choice between TOON and JSON is configurable per session — TOON is preferred for token efficiency (30-60% reduction), JSON for interoperability

**Automatic Repository Clone and Mount (FR-846–857)** [DRAFT]:

- **FR-846**: System MUST provide a `clone_repo` VFS tool that clones a Git repository by URL into the storage layout's repo cache directory (`$CACHE/<product>/repos/<owner>/<repo>/`) and mounts it into the active `CompositeProvider`
- **FR-847**: `clone_repo` MUST accept parameters: `url` (required), `ref` (optional branch/tag/commit, default: HEAD), `index` (optional bool, default: false — whether to trigger semantic indexing after clone), `mount_path` (optional override for the VFS mount point, default: `/repos/<owner>/<repo>/`)
- **FR-848**: After cloning and mounting, all existing VFS tools (read, grep, tree, glob, find, head, tail, stat, diff, semantic_search) MUST work on the mounted repository path without any additional configuration
- **FR-849**: If the repository has already been cloned at the same cache path, `clone_repo` MUST update it via `git fetch` + checkout of the requested ref rather than re-cloning from scratch
- **FR-850**: `clone_repo` MUST use the ambient Git credential configuration (SSH agent, credential helpers, `.netrc`) for authentication — synwire MUST NOT prompt for or store Git credentials
- **FR-851**: `clone_repo` MUST support `depth` parameter for shallow clones (`git clone --depth N`) to reduce clone time and disk usage for large repositories where full history is not needed
- **FR-852**: When `index: true` is specified, `clone_repo` MUST trigger the semantic indexing pipeline (FR-738) on the cloned directory after checkout completes, returning an `IndexHandle` for status polling
- **FR-853**: System MUST provide a `RepoFetchDetector` middleware that monitors `web_fetch` / HTTP tool calls for patterns matching GitHub raw content URLs (`raw.githubusercontent.com/<owner>/<repo>/*`, `github.com/<owner>/<repo>/blob/*`). After a configurable threshold (default: 3) of fetches from the same repository within one session, it MUST emit a `PromptSuggestion` event recommending `clone_repo`
- **FR-854**: `RepoFetchDetector` MUST extract owner and repository name from various GitHub URL formats (HTTPS clone URL, raw content URL, blob URL, API URL) and normalise them to a canonical `<owner>/<repo>` identifier
- **FR-855**: System MUST provide a `repo_gc(max_age_days)` operation that removes cloned repositories from the cache that have not been accessed (mounted or pulled) within the specified period
- **FR-856**: `StorageLayout` MUST provide a `repos_cache() -> PathBuf` accessor for the cloned repository cache directory, and `repo_cache(owner, repo) -> PathBuf` for a specific repository
- **FR-857**: Cloned repository mounts MUST be recorded in the session state so that on session resume, previously-cloned repositories are re-mounted (but not re-cloned if the cache still exists)

**Cross-Project Code References (FR-840–845)** [DRAFT]:

- **FR-840**: System MUST provide a global cross-project code graph (`global/xrefs/`) linking symbols across project boundaries. When project B imports and calls function `foo::bar` from locally-indexed project A, an inter-project edge connects B's call site to A's definition
- **FR-841**: Cross-project edges MUST be built opportunistically during per-project indexing: when the dependency index (FR-835) shows that a dependency is also a locally-indexed project, the indexer resolves import references against the dependency's symbol table
- **FR-842**: System MUST provide a `xref_query(symbol, direction)` operation returning cross-project references: "which other projects call this function?" (consumers) and "where is this imported symbol defined?" (provider). Results include project ID, file, line, and symbol context
- **FR-843**: When a symbol's definition changes in project A, the system MUST be able to identify all cross-project call sites in other indexed projects that reference it, enabling impact analysis across the local project portfolio
- **FR-844**: Cross-project references MUST be invalidated when either the provider or consumer project's index is updated — stale cross-references MUST be marked and lazily rebuilt on next query
- **FR-845**: Cross-project code graph MUST NOT require all projects to be indexed simultaneously — edges are built incrementally as projects are individually indexed, and missing projects produce dangling references that resolve when the project is later indexed

**Dataflow-Guided Retrieval (FR-803–805)** [DRAFT]:

- **FR-803**: System MUST provide a dataflow retrieval mode that traces data dependencies (variable origins and consumers) across function boundaries
- **FR-804**: Dataflow analysis MUST leverage LSP capabilities where available (e.g., type hierarchy, call hierarchy) and fall back to tree-sitter heuristics otherwise
- **FR-805**: Dataflow results MUST include the chain of transformations (which functions/assignments the data passes through) with file and line references

**Multi-Server MCP Client (FR-916–925)**:

- **FR-916**: System MUST provide a `MultiServerMcpClient` that accepts a named map of server connections and manages simultaneous connections to all configured MCP servers
- **FR-917**: `MultiServerMcpClient` MUST establish connections to all servers concurrently (not sequentially) and report per-server connection status
- **FR-918**: System MUST provide a WebSocket transport variant for MCP connections, configured with `url` and `headers`
- **FR-919**: `create_session()` MUST accept any `Connection` variant (Stdio, SSE, StreamableHttp, WebSocket) and return an `McpClientSession` with guard-based cleanup ensuring proper teardown on early returns or panics
- **FR-920**: `get_tools()` MUST load tools from one or all connected servers, returning `Vec<Box<dyn Tool>>`, using cursor-based pagination with a 1000-page safeguard cap
- **FR-921**: When `tool_name_prefix` flag is set, tool names MUST be prefixed as `{server_name}_{tool_name}` with server names sanitised to valid identifiers
- **FR-922**: `MultiServerMcpClient` MUST support per-transport and per-tool timeouts
- **FR-923**: HTTP client behaviour MUST be customisable via rmcp's `StreamableHttpClient` trait, supporting Bearer and Basic authentication schemes
- **FR-924**: System MUST support both per-transport and per-tool timeout configurations
- **FR-925**: `MultiServerMcpClient` MUST monitor server health and exclude unhealthy servers from tool discovery until reconnection succeeds

**MCP Tool Conversion (FR-926–932)**:

- **FR-926**: `convert_mcp_tool_to_synwire_tool()` MUST map an MCP tool definition to a Synwire `Tool`, returning `(content, artifact)` with MCP annotations carried through as metadata
- **FR-927**: `to_mcp_tool()` MUST convert a Synwire tool to an MCP tool definition, validating the `args_schema` and returning an error if injected arguments are present
- **FR-928**: Content type conversion MUST map MCP Text, Image, ResourceLink, and EmbeddedResource to Synwire representations directly
- **FR-929**: AudioContent from MCP MUST return `UnsupportedContent` since Synwire does not yet model audio
- **FR-930**: When MCP sets the `isError` flag on a tool result, conversion MUST raise a `ToolException`
- **FR-931**: JSON Schema validation MUST be performed on tool arguments before invocation, rejecting malformed arguments with a `SchemaValidation` error
- **FR-932**: MCP tool schemas MUST be represented as `serde_json::Value`, avoiding the need for a full JSON Schema type system

**MCP Resources and Prompts (FR-933–937)**:

- **FR-933**: `get_resources()` MUST load MCP resources as `McpBlob` equivalents, excluding dynamic resources
- **FR-934**: Supporting functions `convert_mcp_resource_to_blob()`, `load_mcp_resources()`, and `get_mcp_resource()` MUST be provided for individual and batch resource retrieval
- **FR-935**: `get_prompt()` MUST retrieve an MCP prompt and convert it to Synwire `Message` types
- **FR-936**: `convert_mcp_prompt_message()` MUST handle role-based mapping and multi-content support, translating MCP prompt message structure to Synwire message model
- **FR-937**: Prompt and resource operations MUST use the same cursor-based pagination with 1000-page cap as tool listing

**Tool Call Interceptors (FR-938–942)**:

- **FR-938**: System MUST provide a `ToolCallInterceptor` trait following an onion/middleware pattern for composable tool call wrapping
- **FR-939**: Each interceptor MUST receive an `McpToolCallRequest` containing tool `name`, `args`, `server_name`, `headers`, and runtime context
- **FR-940**: Interceptors MUST be able to return an `McpToolCallResult` (union of `CallToolResult`, `ToolMessage`, or `Command`) to short-circuit the chain
- **FR-941**: Interceptor chain MUST execute in correct onion order with outer interceptors seeing both request and response
- **FR-942**: Interceptors MUST be panic-safe — a failing interceptor MUST NOT corrupt the call chain

**MCP Callbacks (FR-943–945)**:

- **FR-943**: `McpCallbacks` MUST provide slots for LoggingMessage, Progress, and Elicitation callback types
- **FR-944**: LoggingMessage callbacks MUST deliver server-side log output to the client
- **FR-945**: Progress callbacks MUST deliver progress notifications for long-running operations with percentage and message

**Tool Classification and Output (FR-946–950)**:

- **FR-946**: System MUST provide a `ToolCategory` enum with variants `Builtin`, `Custom`, `Mcp`, `Remote`, and `WorkflowAsTool`
- **FR-947**: `ToolOutput` MUST be extended with `content_type: ToolContentType`
- **FR-948**: `ToolContentType` MUST define variants for `Text`, `Image`, `File`, and `Json`
- **FR-949**: System MUST provide a `ToolKind` enum classifying tools by operational nature: `read`, `edit`, `search`, `execute`, or `other`
- **FR-950**: `ToolKind` MUST be queryable by permission UIs to communicate tool impact to users

**Tool Providers (FR-951–955)**:

- **FR-951**: System MUST provide a `ToolProvider` trait with `discover_tools()` and `get_tool()` methods
- **FR-952**: `StaticToolProvider` MUST expose a fixed set of tools configured at construction
- **FR-953**: `McpToolProvider` MUST be backed by `MultiServerMcpClient` and delegate tool discovery to connected MCP servers
- **FR-954**: `CompositeToolProvider` MUST aggregate multiple providers into a single interface with configurable name collision resolution
- **FR-955**: All `ToolProvider` implementations MUST be `Send + Sync` and support concurrent `discover_tools()` calls

**Tool Operational Controls (FR-956–963)**:

- **FR-956**: System MUST support per-tool timeout with `timeout_behavior` set to either `ReturnError` or `RaiseException`
- **FR-957**: System MUST support `is_enabled` predicate controlling whether a tool is included in the LLM schema — disabled tools are omitted entirely
- **FR-958**: System MUST support `max_usage_count` capping per-session tool invocations, returning `ToolUsageLimitExceeded` when exceeded
- **FR-959**: Tool names MUST match `^[a-zA-Z0-9_-]{1,64}$`, enforced at construction time
- **FR-960**: `ToolNode` MUST truncate results exceeding `max_result_size` (default 100 KB)
- **FR-961**: Tool argument schemas MUST be validated before invocation using JSON Schema validation
- **FR-962**: System MUST provide argument validation that catches type mismatches, missing required fields, and extra fields before the tool function is called
- **FR-963**: All tool operational controls MUST be configurable per-tool, not just globally

**Proc-Macro Tool Generation (FR-964–967)**:

- **FR-964**: The `#[tool]` proc-macro MUST generate a `Tool` implementation from an async function, including name, description, JSON Schema, and invocation wrapper
- **FR-965**: The macro MUST derive the tool name from the function name and accept an optional `description` attribute
- **FR-966**: The macro MUST generate a JSON Schema from the function's parameter types
- **FR-967**: The macro MUST support functions returning `Result<ToolOutput, ToolError>` and automatically handle serialisation/deserialisation of arguments

**Agents as Tools (FR-968–970)**:

- **FR-968**: `CompiledGraph::as_tool()` MUST wrap a compiled graph as a `Tool`, accepting the graph's input state type as tool input and returning its output state
- **FR-969**: A `CompiledGraph` MUST be usable directly as a node within another `StateGraph`, enabling hierarchical agent architectures
- **FR-970**: When a graph-as-tool errors, the error MUST propagate to the outer graph as a tool error with full context

**MCP Adapter Error Types (FR-971–976)**:

- **FR-971**: The MCP adapters crate MUST define `ServerNotFound` error for unknown server names in multi-server operations
- **FR-972**: The MCP adapters crate MUST define `Transport` error for connection or protocol-level errors
- **FR-973**: The MCP adapters crate MUST define `ConnectionFailed` error for initial connection establishment failures
- **FR-974**: The MCP adapters crate MUST define `Timeout` error for operations exceeding configured timeout
- **FR-975**: The MCP adapters crate MUST define `ToolNotFound` error for unknown tool names on target servers
- **FR-976**: The MCP adapters crate MUST define `SchemaValidation` error for tool arguments failing schema validation

### Key Entities

- **Directive**: Typed effect description returned by agent nodes (Emit, SpawnAgent, StopChild, Schedule, RunInstruction, Cron, Stop, SpawnTask, StopTask, Custom)
- **ExecutionStrategy**: Controls how agent orchestrates actions (DirectStrategy for immediate sequential execution, FsmStrategy for state-constrained workflows)
- **Plugin**: Runner-scoped component with lifecycle hooks and isolated state slice identified by `PluginStateKey`
- **Backend**: Implements file, shell, git, HTTP, process, and archive operations with persistent working directory state (ephemeral state, persistent store, filesystem, local shell, git repository, HTTP client, process manager, archive handler, composite routing)
- **GrepOptions**: Enhanced search configuration (context lines, case sensitivity, file type filters, match limits, binary handling, line numbers, invert match, count mode)
- **GitProvider**: Version control operations scoped to specific repositories (status, diff, log, commit, push, pull, branch management) — removed from VFS protocol, available as standalone tool/middleware
- **HttpProvider**: Web request operations with timeout, SSL validation, headers, redirects (GET, POST, PUT, DELETE, custom methods) — removed from VFS protocol, available as standalone tool/middleware
- **ProcessProvider**: Process lifecycle management (list, kill, spawn background, job control, foreground/background) — part of sandbox module
- **ArchiveProvider**: Archive creation/extraction with compression (tar, gzip, zip, bzip2) and conflict resolution policies — part of sandbox module
- **ApprovalCallback**: User approval mechanism for risky operations providing operation context and receiving user response, with decision variants Allow, Deny, AllowAlways, Abort
- **PermissionMode**: Named presets controlling tool approval behavior (Default, AcceptEdits, PlanOnly, BypassAll, DenyUnauthorized)
- **PipelineProvider**: Command pipeline composition with stream redirection (stdin, stdout, stderr) and error propagation — part of sandbox module
- **Middleware**: Stackable component adding tools, modifying prompts, or transforming state (filesystem, git, HTTP, process, archive, pipeline, environment, summarization, prompt caching, patch tool calls)
- **Agent**: Builder API for agent construction with typed dependencies, output modes, model selection, sandbox config, debug mode, env vars, and cwd
- **Signal**: Incoming message or event routed through three-tier priority system (strategy, agent, plugin)
- **RunContext**: Execution context carrying dependencies, model reference, retry count, usage, metadata
- **HookRegistry**: Registry for lifecycle hooks (PreToolUse, PostToolUse, PostToolUseFailure, Notification, SubagentStart, SubagentStop, PreCompact, PostCompact, SessionStart, SessionEnd) with matcher patterns and timeout
- **Session**: Persistable agent conversation with checkpoint support, metadata (tags, title), fork/rewind capability
- **Usage**: Per-turn and cumulative token tracking with cost estimation (input, output, cache read, cache creation, cost USD)
- **ModelInfo**: Runtime model metadata including capabilities (vision, tool-calling, effort levels), context window size, and billing multiplier
- **McpServerConfig**: External MCP server connection configuration supporting stdio, HTTP, SSE, and in-process transports
- **SandboxConfig**: Agent-level isolation settings for network, filesystem scope, and command restrictions
- **AgentError**: Top-level error taxonomy with subtypes for model (auth, billing, rate-limit), tool, strategy, middleware, directive, backend, session, panic, and budget errors
- **ThinkingConfig**: Reasoning configuration with modes: adaptive, enabled (with token budget), disabled
- **Vfs**: Virtual filesystem trait replacing `Vfs` — filesystem-like interface over heterogeneous data sources with Linux coreutils-style operations
- **VfsCapabilities**: Bitflags declaring which VFS operations a provider supports (30 capability flags)
- **MemoryProvider**: Ephemeral in-memory VFS provider (replacing `MemoryProvider`)
- **LocalProvider**: Real filesystem VFS provider with path traversal protection (replacing `LocalProvider`)
- **CompositeProvider**: Mount-routing VFS provider delegating to sub-providers by path prefix (replacing `CompositeProvider`)
- **ReadGuard**: Session-scoped tracker enforcing "must read before edit" policy and stale-read detection
- **Chunker**: Tree-sitter AST-aware code splitter producing semantic `Document` chunks with metadata (file, language, lines, symbol)
- **SemanticIndex**: Pipeline orchestrating walk → chunk → embed → store for semantic code search with incremental re-indexing
- **LspClient**: Language Server Protocol client managing server lifecycle, document sync, and diagnostics caching
- **DapClient**: Debug Adapter Protocol client managing debug sessions (breakpoints, stepping, evaluation)
- **CodeGraph**: Directed graph of code entities (files, classes, functions) and typed edges (calls, imports, contains, inherits) for multi-hop navigation
- **FileSkeleton**: Compact structural summary of a source file containing only definition signatures without bodies
- **HierarchicalNarrowing**: Three-phase localization pipeline: directory tree → file skeletons → targeted line ranges
- **HybridSearch**: Combined BM25 (lexical) + vector (semantic) search with configurable alpha weighting
- **SBFL**: Spectrum-Based Fault Localization — ranking functions by suspiciousness based on test pass/fail coverage data (Ochiai metric)
- **ExperiencePool**: Persistent repository memory storing edit→file associations, file summaries, and issue→resolution mappings across sessions
- **MCTSSearch**: Monte Carlo Tree Search strategy for exploring multiple localization/repair trajectories with value-function scoring
- **CommunityState**: Persistent hierarchical community partition state (from hit-leiden) supporting incremental updates via delta edges, serialisable for checkpoint persistence
- **CommunitySearch**: Multi-resolution search over LLM-generated community summaries — search at coarse level first, drill into member symbols for precise localization
- **StorageLayout**: Product-scoped persistent storage coordinator computing paths for all subsystems (sessions, indices, graphs, communities, experience pools) using platform conventions, with configurable product name and root override
- **ProjectId**: Stable project identifier derived from Git first-commit hash (or path hash fallback) that survives directory moves and works across machines
- **RepoId**: Stable repository family identifier from Git first-commit hash — shared across all worktrees of the same repo. Used to scope the singleton index coordinator
- **WorktreeId**: Specific working copy identifier (`RepoId` + worktree root path hash) — each worktree has its own index since file contents differ between branches
- **SynwireDaemon**: Singleton background process per product managing all repos, worktrees, and cloned repos. Owns the embedding model, all file watchers, all indexing pipelines, and the global tier (registry, deps, xrefs, experience). MCP servers connect via Unix domain socket as thin stdio↔UDS proxies. Auto-started by first MCP server, survives editor closes via 5-minute grace period, independently startable for pre-warming
- **NativeConcurrency**: Inter-process data sharing strategy using each backend's built-in concurrency (SQLite WAL, LanceDB concurrent reads, tantivy IndexReader/Writer, atomic rename for blobs) instead of external file locks
- **StorageMigration**: Per-subsystem schema migration framework with version tracking and copy-then-swap atomic upgrades
- **ProjectRegistry**: Global index of all known projects with metadata (ID, path, remote URL, last accessed, tags) enabling cross-project discovery
- **DependencyIndex**: Global cross-project dependency graph mapping projects to their declared dependencies (from manifest files), queryable by dependency or by project
- **GlobalExperiencePool**: Cross-project experience store for generalised fix patterns, searched as fallback when project-local pool has no matches
- **CrossProjectGraph**: Global inter-project code reference graph linking symbols across project boundaries (call sites in project B → definitions in project A), built opportunistically during indexing when dependencies are locally indexed projects
- **CloneRepo**: VFS tool that clones a Git repository by URL, mounts it as a `LocalProvider` in the `CompositeProvider`, and optionally triggers semantic indexing — making the entire repository immediately searchable via all VFS tools
- **RepoFetchDetector**: Middleware that monitors HTTP fetch patterns, detects repeated file-by-file fetches from the same GitHub repository, and suggests cloning the entire repository for efficient local access
- **SynwireMcpServer**: Standalone binary exposing synwire VFS, search, code graph, LSP, and DAP tools as an MCP server, usable with Claude Code, Copilot, Cursor, and any MCP-compatible client via stdio transport
- **ToolSearchIndex**: Framework-level tool discovery engine in `synwire-core`. Pre-computes embeddings from tool descriptions + example queries, provides `tool_search(query, top_k)` for semantic retrieval and namespace browsing, tracks loaded schemas for deduplication. Used by the synwire agent runtime, MCP server, and available to any third-party agent
- **SamplingProvider**: Trait abstracting LLM access for tool-internal use (community summaries, hierarchical narrowing ranking, experience pool summaries). Two implementations: MCP sampling (delegates to client) and direct model invocation (standalone mode). Degrades gracefully when unavailable
- **AgentSkill**: A directory containing a `SKILL.md` file following the [Agent Skills specification](https://agentskills.io/specification) — YAML frontmatter (name, description, optional license/compatibility/metadata/allowed-tools) plus Markdown instructions, with optional `scripts/`, `references/`, `assets/` subdirectories. Synwire extends with optional `runtime` field for embedded execution (Lua/Rhai/WASM)
- **SKILL.md**: The required file in every agent skill directory — contains YAML frontmatter for metadata and Markdown body for instructions. Loaded progressively: metadata at startup, full body on activation, referenced files on demand
- **SkillLoader**: Component in `synwire-agent-skills` crate that discovers skills in `$DATA/<product>/skills/` (global) and `.<product>/skills/` (project-local, configurable), validates `SKILL.md` frontmatter against the Agent Skills spec, and registers skills with progressive disclosure
- **`MultiServerMcpClient`**: Manages connections to N named MCP servers simultaneously, aggregates tools from all servers, supports health monitoring and reconnection, provides tool name prefixing to avoid collisions
- **Connection**: Enum of MCP transport variants — Stdio (child process), SSE (legacy), StreamableHttp (current MCP spec), WebSocket — each with transport-specific configuration
- **`McpClientSession`**: Guard-scoped session to a single MCP server, ensuring cleanup on drop. Created by `create_session()` from any `Connection` variant
- **`ToolCallInterceptor`**: Trait for composable onion/middleware around MCP tool calls. Receives `McpToolCallRequest`, returns `McpToolCallResult`. Panic-safe
- **`McpToolCallRequest`**: Request context for interceptors containing tool `name`, `args`, `server_name`, `headers`, and runtime context
- **`McpToolCallResult`**: Union result from interceptors — `CallToolResult`, `ToolMessage`, or `Command`
- **`McpCallbacks`**: Callback slots for LoggingMessage, Progress, and Elicitation notifications from MCP servers
- **`ToolCategory`**: Classification enum — `Builtin`, `Custom`, `Mcp`, `Remote`, `WorkflowAsTool`
- **`ToolKind`**: Operational nature enum — `read`, `edit`, `search`, `execute`, `other`. Used by permission UIs to communicate impact
- **`ToolContentType`**: Output content type — `Text`, `Image`, `File`, `Json`
- **`ToolProvider`**: Trait for tool discovery with `discover_tools()` and `get_tool()`. Implementations: `StaticToolProvider` (fixed set), `McpToolProvider` (MCP-backed), `CompositeToolProvider` (aggregated)
- **`McpAdapterError`**: Error type with variants `ServerNotFound`, `Transport`, `ConnectionFailed`, `Timeout`, `ToolNotFound`, `SchemaValidation`

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-097**: Developers can write agent unit tests that verify directive output without executing any side effects (zero filesystem access, zero subprocess execution, zero network calls)
- **SC-098**: Middleware can prevent directive execution by implementing `DirectiveFilter` (e.g., blocking SpawnAgent in sandboxed mode) with 100% suppression success rate
- **SC-099**: All directive variants successfully round-trip through serialization/deserialization preserving all data and enabling replay analysis
- **SC-100**: FSM execution strategy rejects invalid state transitions with explicit error messages containing current state and attempted action 100% of the time
- **SC-101**: Identical agent logic produces identical final results under DirectStrategy and FsmStrategy when all transitions are valid
- **SC-102**: Two plugins writing to their states concurrently maintain complete state isolation with zero cross-plugin state interference
- **SC-103**: Signal routing resolves strategy-level routes before agent and plugin routes in 100% of cases
- **SC-013**: All VFS provider implementations pass conformance test suite with 100% success rate
- **SC-013a**: Bash-style commands (ls, cd, grep, rm, cp, mv) correctly translate to backend operations with familiar shell semantics across all backend types
- **SC-013b**: GitProvider successfully performs version control operations (status, diff, commit, push) against test repositories with correct git semantics
- **SC-013c**: HttpProvider successfully performs web requests (GET, POST) with proper header handling, timeout behavior, and error reporting
- **SC-013d**: GitProvider rejects operations outside scoped repository path with scope violation errors 100% of the time
- **SC-013e**: HttpProvider follows redirects up to configured limit and returns final URL and response correctly
- **SC-014**: Middleware stack assembles without type conflicts and invokes middleware in declared order for 100% of configurations
- **SC-014a**: Approval gates successfully intercept risky operations and only execute after user approval with zero unauthorized executions
- **SC-014b**: Approval requests include complete operation context (type, resources, description) enabling informed user decisions
- **SC-014c**: Denied approval requests fail immediately without executing operations and return user-denied error
- **SC-015**: Enhanced grep successfully searches with context lines (-C 3), case-insensitive (-i), file type filters, and match limits returning accurate results with line numbers
- **SC-016**: Working directory state persists across operations with cd changing state and relative paths resolving from current directory 100% of the time
- **SC-017**: ProcessProvider successfully lists processes, kills processes by PID, spawns background jobs, and manages job control (foreground/background) with correct state tracking
- **SC-018a**: ArchiveProvider successfully creates and extracts tar.gz, zip, and bzip2 archives preserving file structure and permissions
- **SC-019**: Command pipelines correctly pipe output between stages with proper error propagation and stream redirection (stdin, stdout, stderr, 2>&1)
- **SC-020**: Grep correctly handles binary files by skipping them by default unless binary-files=text option is specified
- **SC-021**: Archive extraction with file conflicts applies configured policy (skip/overwrite/rename/error) consistently
- **SC-022**: Pipeline timeout enforces per-stage limits and returns timeout error for hanging stages without blocking indefinitely
- **SC-018**: Test suite achieves at least 80% line coverage across all new crates (`synwire-agent`, new modules in `synwire-core`)
- **SC-019**: Agent runtime contains zero `unsafe` blocks
- **SC-020**: All public agent types are `Send + Sync` enabling concurrent usage
- **SC-026**: Developers can create working tool-calling agent with structured output in 5 or fewer lines using `Agent::builder()` API (excluding import statements, async runtime setup, and error handling — counting only builder chain and invocation)
- **SC-027**: Agent API automatically selects optimal `OutputMode` per model without developer intervention
- **SC-033**: Runner catches panics in agent or middleware code, logs the panic payload and backtrace at error level, emits an `Error` event to the stream, and returns `AgentError::Panic` to the caller without crashing the process
- **SC-058**: `max_turns` configuration successfully limits agent conversation to configured turn count, emitting `TurnComplete` with `max_turns_exceeded` reason
- **SC-059**: Session resume successfully restores full agent state including conversation history, plugin state, and working directory from checkpoint
- **SC-060**: Session fork creates an independent copy with history preserved up to the fork point, and subsequent operations on the fork do not affect the original
- **SC-061**: `PreToolUse` hook can reject a tool invocation, and the rejection is returned to the model as a tool error with the hook's reason
- **SC-062**: `PreToolUse` hook can modify tool arguments, and the modified arguments are used for tool execution
- **SC-063**: MCP servers connected via stdio transport provide tools that are callable by the agent identically to native tools
- **SC-064**: Dynamic model switching mid-conversation preserves conversation history and agent state
- **SC-065**: Fallback model is automatically used when primary model returns rate-limit error, transparent to the agent logic
- **SC-066**: `max_budget` cost limit stops agent execution with `BudgetExceeded` when cumulative cost exceeds the configured threshold
- **SC-067**: Declarative permission rules correctly match tool name patterns and apply the configured behavior (allow/deny/ask)
- **SC-068**: Force stop cancels in-flight tool executions within 1 second via cooperative cancellation
- **SC-069**: Hook timeout causes the hook to be skipped (not the agent to fail), with a warning logged
- **SC-101a**: "Identical final results" under both strategies means structurally equal `serde_json::Value` output — both strategies produce the same JSON when serialised. Map key ordering does not matter (comparison uses semantic equality). Floating-point values are compared with epsilon tolerance
- **SC-070**: `LocalShellProvider` command injection test: user-provided arguments containing shell metacharacters (`; | & $ \``) MUST NOT be interpreted by the shell — they are passed as literal strings to the subprocess
- **SC-071**: Checkpoint atomicity test: simulated crash during checkpoint write MUST leave the previous checkpoint intact and resumable
- **SC-072**: Plugin state schema migration test: session resume with an incompatible plugin state schema resets that plugin to defaults without affecting other plugins or agent state
- **SC-073**: `DirectiveFilter::Reject` surfaces rejection reason to the model as a tool error, enabling the model to see why its directive was blocked
- **SC-074**: Subagent privilege constraint: child agent with `PermissionMode::BypassAll` spawned by parent with `PermissionMode::Default` MUST be capped at parent's permission level
- **SC-075**: Concurrent tool call event interleaving: events from different tool call IDs may interleave, but each ID's internal sequence (Start→Delta→End→Progress→Result) is preserved
- **SC-076**: `FakeModel` in test-utils can be configured to return rate-limit errors, authentication errors, and streaming sequences, enabling all error-path tests without network access
- **SC-077**: `FakeClock` in test-utils enables deterministic testing of `Schedule`, `Cron`, and `ApprovalTimeout` without wall-clock dependency
- **SC-078**: Builder validation: setting both `allowed_tools` and `excluded_tools` returns a clear error at `build()` time, not a runtime panic

**VFS & Semantic Search (implemented)**:

- **SC-079**: VFS ReadGuard prevents edits to files that have not been read in the current session, returning a clear error message
- **SC-080**: VFS stale-read detection correctly identifies files modified externally between read and edit, preventing data loss from concurrent modification
- **SC-081**: All VFS providers (MemoryProvider, LocalProvider, CompositeProvider) pass the VFS conformance test suite with 100% success rate for advertised capabilities
- **SC-082**: AST-aware chunking produces separate `Document`s for each top-level definition in supported languages, with correct symbol name and line range metadata
- **SC-083**: Semantic search returns relevant code snippets for conceptual queries (not just keyword matches) with similarity scores above a configurable threshold
- **SC-084**: Incremental re-indexing skips unchanged files (verified via content hash), re-indexing only modified files
- **SC-085**: File watcher detects changes to indexed files and triggers automatic re-indexing without manual intervention
- **SC-086**: LSP tools are generated only for capabilities the server advertises — no tool is offered that the server cannot fulfil
- **SC-087**: LSP go-to-definition correctly resolves symbol locations across files in multi-file projects
- **SC-088**: DAP breakpoint/step/evaluate cycle works correctly for at least one supported debug adapter
- **SC-089**: Sandbox process isolation prevents spawned processes from accessing files outside their configured scope

**Per-Method Chunking (draft)**:

- **SC-090**: Rust `impl` blocks with N methods produce N separate chunks (not 1), each with `symbol` metadata in `Type::method` format
- **SC-091**: Python/Java/TypeScript class bodies with M methods produce M separate chunks, each with class name as context prefix
- **SC-092**: Top-level functions not inside containers produce chunks identically to current behaviour (no regression)

**File Skeleton & Hierarchical Narrowing (draft)**:

- **SC-093**: File skeleton output is less than 25% of full file token count for files with 10+ definitions
- **SC-094**: Hierarchical narrowing identifies the correct file in the top-3 candidates for at least 70% of localization tasks on a representative test suite
- **SC-095**: Hierarchical narrowing identifies the correct function in the top-3 candidates for at least 50% of localization tasks

**Code Graph (draft)**:

- **SC-096**: Code graph correctly captures cross-file call edges — function A in file X calling function B in file Y produces an edge from A to B
- **SC-100a**: `graph_search` with 2+ hops finds related code that flat semantic search misses, measured on a test suite of multi-file bugs
- **SC-101b**: Graph construction is incremental — modifying one file recomputes only that file's edges, not the entire graph

**Hybrid Search (draft)**:

- **SC-102a**: Hybrid search with alpha=0.5 finds both exact identifier matches (BM25 strength) and semantically similar code (vector strength) that neither approach finds alone
- **SC-103a**: Hybrid search at alpha=1.0 produces identical results to pure BM25; at alpha=0.0 produces identical results to pure vector search

**Test-Guided Localization (draft)**:

- **SC-104**: SBFL Ochiai scoring correctly ranks the buggy function as the most suspicious when coverage data shows it is executed by failing tests but not by passing tests
- **SC-105**: When no coverage data is available, the system falls back to semantic search without errors

**Repository Memory (draft)**:

- **SC-106**: Edit associations recorded in session A are queryable in session B without re-indexing
- **SC-107**: Experience pool queries return relevant results within 500ms for repositories with up to 10,000 recorded associations

**Advanced Search (draft)**:

- **SC-108**: MCTS with depth 3 identifies correct localization more often than depth 1 on a representative test suite
- **SC-109**: Dynamically-created tools pass sandbox and permission checks identically to native tools
- **SC-109a**: A Lua agent-skill producing the same output as a Rhai agent-skill for identical inputs, verifying runtime portability
- **SC-109b**: An Extism WASM skill runs sandboxed — attempts to access VFS operations not listed in its manifest permissions are denied
- **SC-109c**: Skills placed in `$DATA/<product>/skills/` are auto-discovered and their tools appear in the MCP server tool list on startup
- **SC-109d**: A skill manifest with an invalid schema or missing entrypoint is rejected at load time with a clear error, not at runtime
- **SC-110**: Dataflow retrieval traces variable origins across at least 2 function boundaries in supported languages

**GraphRAG Community Detection (draft)**:

- **SC-111**: Community detection groups strongly-connected code clusters (e.g., modules with high internal call density) into distinct communities with modularity score > 0.3
- **SC-112**: Incremental community update after a single file change is at least 10x faster than full reclustering on a codebase with 100,000+ symbols (Linux kernel scale)
- **SC-113**: `community_search` finds relevant communities via summary search and returns member symbols that flat semantic search misses (measured on a test suite of cross-module queries)
- **SC-114**: `CommunityState` round-trips through `into_parts()`/`from_parts()` and subsequent incremental updates produce identical results to continuous state
- **SC-115**: Community summaries are regenerated only for communities whose members were affected by file changes — unaffected community summaries remain cached

**Persistent Storage Layout (draft)**:

- **SC-116**: Two products with different names produce fully isolated storage paths with zero overlap — verified by comparing all path accessors
- **SC-117**: `ProjectId` from a Git repository is identical before and after moving the repository to a different directory path
- **SC-118**: `ProjectId` for the same repository is identical on two different machines (same first commit hash)
- **SC-119**: `SqliteSaver` and `SemanticIndex` receive paths from `StorageLayout` and function identically to current hardcoded paths — zero behavioural regression
- **SC-120**: Native backend concurrency (SQLite WAL, LanceDB, tantivy) prevents two concurrent MCP server instances from corrupting the same project's data — verified by parallel write + read test with zero corruption and zero blocked reads
- **SC-121**: Storage migration detects version mismatch and runs migration successfully, with failed migration leaving previous data intact
- **SC-122**: `SYNWIRE_DATA_DIR` environment variable overrides all other path configuration sources
- **SC-123**: `StorageLayout` with `root_override` places all data under a single directory tree — verified by checking no paths escape the override root
- **SC-123a**: Three MCP server instances across two repos share a single daemon process — verified by checking only one embedding model is loaded in total (not 3x) and total inotify watches equal the sum of watched files across repos (not 3x per repo)
- **SC-123b**: Two worktrees of the same repo produce distinct `WorktreeId`s but share a `RepoId` — searches on worktree A return results from worktree A's index, not worktree B's
- **SC-123c**: When the daemon process exits, the next MCP server instance restarts it within 5 seconds, reusing existing on-disk indices without re-indexing
- **SC-123d**: Daemon remains running for the grace period (5 minutes) after last client disconnects, and a reconnecting client reuses it without re-loading the embedding model
- **SC-123e**: `clone_repo` via MCP server triggers daemon-side clone + indexing — the cloned repo's index is available to all connected MCP servers, not just the one that requested the clone
- **SC-123f**: Cross-project `xref_query` works across repos managed by the same daemon — symbols in repo A called from repo B are findable when both are indexed
- **SC-124**: Global dependency index correctly answers "which of my projects depend on library X?" across 5+ registered projects, returning project names and version constraints
- **SC-125**: Global experience pool returns relevant cross-project fix patterns when the project-local pool has no matches — verified by recording a pattern in project A and querying it from project B
- **SC-126**: Project registry is updated on every project access and accurately reflects last-accessed timestamps and known project set
- **SC-127**: Cross-project `xref_query` correctly identifies call sites in project B that reference a function defined in locally-indexed project A
- **SC-128**: After changing a function signature in project A and re-indexing, `xref_query` for that symbol returns all consuming call sites across other indexed projects for impact analysis
- **SC-129**: Cross-project references are built incrementally — indexing project A then project B produces the same cross-references as indexing both simultaneously

**Automatic Repository Clone and Mount (draft)**:

- **SC-130**: `clone_repo` clones a public GitHub repository and mounts it such that `grep`, `tree`, and `read` work on the mounted path within 30 seconds for a repository under 100MB
- **SC-131**: `RepoFetchDetector` correctly identifies 3+ fetches from the same GitHub repository across different URL formats (raw, blob, API) and emits a clone suggestion
- **SC-132**: `repo_gc` reclaims disk space by removing repos not accessed within the configured period, with zero effect on recently-accessed repos
- **SC-133**: Session resume re-mounts previously-cloned repositories from cache without triggering a fresh clone
- **SC-134**: `clone_repo` with `index: true` triggers semantic indexing that completes and enables `semantic_search` on the cloned repository

**Standalone MCP Server (draft)**:

- **SC-135**: MCP server binary starts and connects successfully to Claude Code via stdio transport, with all synwire tools appearing in the tool list
- **SC-136**: Semantic search, grep, tree, and skeleton tools return correct results when invoked from Claude Code against an indexed project
- **SC-137**: MCP server binary is installable via `cargo install synwire-mcp-server` and runs as a single binary with no external runtime dependencies beyond Git
- **SC-138**: Two concurrent MCP server instances (separate processes, same project) querying the same indexed project receive correct results with no corruption — one instance can search while the other indexes without blocking
- **SC-139**: Long-running operations (indexing, cloning) emit MCP progress notifications that clients can display
- **SC-140**: MCP server with `--product-name` flag produces fully isolated storage from the default, verified by checking storage paths
- **SC-140a**: Community summary generation via MCP sampling produces a natural language summary that describes the community's purpose, given a community with 10+ related symbols
- **SC-140b**: When MCP sampling is unavailable, community summaries gracefully degrade to member lists and hierarchical narrowing returns unranked candidates — no errors or failures
- **SC-140c**: Hierarchical narrowing with sampling ranks the correct file in top-3 more often than without sampling (alphabetical fallback)
- **SC-141**: MCP server logs appear on stderr at info level by default and do not corrupt the MCP JSON-RPC protocol on stdout
- **SC-142**: Log files are written to `$DATA/<product>/logs/` with daily rotation and are readable for post-mortem analysis after the server process exits
- **SC-143**: Indexing log entries include file count, chunk count, duration, and error details sufficient to diagnose "why is search returning bad results?"
- **SC-144**: Full indexing of a 70,000-file repository (Linux kernel scale) completes without exceeding 2GB RSS memory, using disk-backed storage for vector index, BM25 index, and code graph
- **SC-145**: Semantic search on a 70,000-file indexed repository returns results within 2 seconds
- **SC-146**: Code graph queries (`graph_query`, `graph_search`) on a graph with 1M+ edges complete within 1 second for depth ≤ 3

**Tool Search and Progressive Discovery (draft)**:

- **SC-147**: Initial MCP tool listing for 40+ tools consumes <2,000 tokens (names + descriptions only), compared to >20,000 tokens for full schemas
- **SC-148**: `tool_search` returns the correct tool in the top-3 results for at least 80% of natural language queries on a test suite of 50 representative queries
- **SC-149**: Tool selection accuracy with `tool_search` is measurably higher than with all schemas loaded — verified on a benchmark of multi-tool tasks
- **SC-150**: Namespace browsing via `tool_search("namespace:X")` returns exactly the tools in that namespace with zero false positives
- **SC-151**: `search_progressive` for a multi-step query ("find the definition, refactor it, run tests") returns tools from all three intents (search, edit, test tools) — not just the first intent
- **SC-152**: After recording 100+ tool invocation transitions, the `ToolTransitionGraph` correctly boosts `file.read` after `search.grep` (common sequence) — verified by score comparison with/without graph
- **SC-153**: Seen/unseen adaptive scoring returns novel tools more often than already-loaded tools when both score similarly — verified by mock scenario with 20 loaded + 20 unloaded tools

**MCP Adapters (draft)**:

- **SC-154**: `MultiServerMcpClient` connects to two MCP servers (one stdio, one HTTP) simultaneously and loads tools from both, with all tools available in a unified tool set
- **SC-155**: A Synwire tool round-trips through MCP conversion (`to_mcp_tool()` → `convert_mcp_tool_to_synwire_tool()`) without data loss in name, description, schema, or annotations
- **SC-156**: Each MCP transport variant (Stdio, SSE, StreamableHttp, WebSocket) passes connection establishment and tool call tests
- **SC-157**: Interceptor chain with 3 interceptors executes in correct onion order — verified by recording call sequence and confirming A→B→C→tool→C→B→A
- **SC-158**: All three MCP callback types (LoggingMessage, Progress, Elicitation) deliver notifications with correct context to registered handlers
- **SC-159**: `#[tool]` proc-macro generates a working `Tool` implementation from an async function that passes invocation, schema, and description tests
- **SC-160**: MCP content types (Text, Image, ResourceLink, EmbeddedResource) convert correctly to Synwire representations — AudioContent returns `UnsupportedContent`
- **SC-161**: `CompositeToolProvider` aggregating a `StaticToolProvider` (3 tools) and `McpToolProvider` (2 tools) returns all 5 tools via `discover_tools()`
- **SC-162**: Tool with `timeout: 100ms` and slow operation returns timeout error within 200ms of the configured timeout
- **SC-163**: Tool with `max_usage_count: 3` returns `ToolUsageLimitExceeded` on 4th invocation
- **SC-164**: Tool with invalid name (`"invalid name!"`) is rejected at construction time with name validation error
- **SC-165**: Tool result exceeding 100 KB is truncated by `ToolNode` to the configured `max_result_size`
- **SC-166**: `CompiledGraph::as_tool()` wraps a graph as a tool that can be invoked from another graph, with input/output state correctly passed through
- **SC-167**: JSON Schema validation rejects malformed tool arguments before the tool function is invoked — verified with missing required field and wrong type
- **SC-168**: Cursor-based pagination with a misbehaving server (never-ending pages) terminates at the 1000-page cap without hanging
- **SC-169**: `McpToolProvider` backed by a `MultiServerMcpClient` with 2 servers returns tools from both servers via `discover_tools()`, with `ToolCategory::Mcp` on each
- **SC-170**: `ToolKind` correctly classifies tools — a VFS read tool reports `ToolKind::read`, a VFS write tool reports `ToolKind::edit`
