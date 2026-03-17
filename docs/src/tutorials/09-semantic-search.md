# Tutorial 9: Semantic Search

**Time**: ~1 hour
**Prerequisites**: Rust 1.85+, completion of [Your First Agent](./01-first-agent.md)

> **Background**: [Retrieval-Augmented Generation](https://www.promptingguide.ai/techniques/rag) — semantic search is the retrieval component of RAG. Instead of matching exact text, it finds code by *meaning*.

This tutorial covers three ways to use Synwire's semantic search:

1. **Direct API** — call `LocalProvider` methods from your own code
2. **As a built-in agent tool** — the agent calls `semantic_search` alongside file and shell tools
3. **As a StateGraph node** — semantic search as a stage in a multi-step pipeline

All three approaches use the same underlying pipeline: walk → chunk → embed → store → search. The difference is who controls when and how it runs.

> 📖 **Rust note:** [`Arc<dyn Vfs>`](https://doc.rust-lang.org/std/sync/struct.Arc.html) is a thread-safe reference-counted pointer to a trait object. `Arc` lets multiple tools share one VFS provider without copying it. `dyn Vfs` means "any type implementing the `Vfs` trait" — the concrete type (`LocalProvider`) is erased at runtime.

---

## Step 1: Enable the feature flag

Add the `semantic-search` feature to your `synwire-agent` dependency:

```toml
[dependencies]
synwire-agent = { version = "0.1", features = ["semantic-search"] }
```

This pulls in `synwire-index`, `synwire-chunker`, `synwire-embeddings-local`,
and `synwire-vectorstore-lancedb` — everything needed for local semantic search.

> **Note**: The first time the embedding models are used, fastembed downloads
> ~30 MB from Hugging Face Hub and caches them locally. Subsequent runs load
> from cache with no network access.

---

## Step 2: Create a `LocalProvider` with semantic search

`LocalProvider` is the VFS implementation for local filesystem access. When the
`semantic-search` feature is enabled, it gains `index`, `index_status`, and
`semantic_search` capabilities.

```rust,ignore
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{IndexOptions, SemanticSearchOptions};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = PathBuf::from("/path/to/your/project");
    let vfs = LocalProvider::new(project_root)?;

    // The VFS reports its capabilities — INDEX and SEMANTIC_SEARCH are included:
    let caps = vfs.capabilities();
    println!("Capabilities: {:?}", caps);

    Ok(())
}
```

---

## Step 3: Index a directory

Call `index()` to start building the semantic index. This returns immediately
with an `IndexHandle` — the actual work runs in a background task.

```rust,ignore
let handle = vfs.index("src", IndexOptions {
    force: false,           // reuse cache if available
    include: vec![],        // no include filter (index everything)
    exclude: vec![
        "target/**".into(), // skip build artifacts
        "*.lock".into(),    // skip lock files
    ],
    max_file_size: Some(1_048_576), // skip files over 1 MiB
}).await?;

println!("Indexing started: id={}", handle.index_id);
```

### What happens in the background

1. **Walk**: `synwire-index` recursively traverses the directory, applying your
   include/exclude filters and file size limit.
2. **Chunk**: Each file is split into semantic units. Code files are parsed with
   tree-sitter to extract functions, structs, classes, and other definitions.
   Non-code files use a recursive character splitter.
3. **Embed**: Each chunk is converted into a 384-dimension vector using
   BAAI/bge-small-en-v1.5 (local ONNX inference).
4. **Store**: Vectors are written to a LanceDB table cached on disk.
5. **Watch**: A file watcher starts monitoring for changes to keep the index
   up to date.

---

## Step 4: Wait for indexing to complete

Poll `index_status()` to check progress:

```rust,ignore
use synwire_core::vfs::types::IndexStatus;

loop {
    let status = vfs.index_status(&handle.index_id).await?;
    match status {
        IndexStatus::Pending => println!("Waiting to start..."),
        IndexStatus::Indexing { progress } => {
            println!("Indexing: {:.0}%", progress * 100.0);
        }
        IndexStatus::Ready(result) => {
            println!(
                "Done! {} files indexed, {} chunks produced (cached: {})",
                result.files_indexed,
                result.chunks_produced,
                result.was_cached,
            );
            break;
        }
        IndexStatus::Failed(err) => {
            eprintln!("Indexing failed: {err}");
            return Err(err.into());
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}
```

> 📖 **Rust note:** The `loop { ... break }` pattern runs until `break` is reached. The `match` arms handle each variant of the [`IndexStatus`] enum. Rust's exhaustive matching ensures you handle every possible state — if a new variant is added, the compiler tells you.

For a medium-sized Rust project (~500 files), indexing typically takes 5–30
seconds depending on CPU speed and whether models need downloading.

---

## Step 5: Search by meaning

Now search for code by what it *does*, not what it is *called*:

```rust,ignore
let results = vfs.semantic_search("error handling and recovery logic", SemanticSearchOptions {
    top_k: Some(5),
    min_score: None,          // no minimum score threshold
    file_filter: vec![],      // search all indexed files
    rerank: Some(true),       // enable cross-encoder reranking (default)
}).await?;

for result in &results {
    println!("--- {} (lines {}-{}, score: {:.3}) ---",
        result.file,
        result.line_start,
        result.line_end,
        result.score,
    );
    if let Some(ref sym) = result.symbol {
        println!("Symbol: {sym}");
    }
    if let Some(ref lang) = result.language {
        println!("Language: {lang}");
    }
    // Print the first 200 characters of content
    let preview: String = result.content.chars().take(200).collect();
    println!("{preview}");
    println!();
}
```

### Understanding results

Each `SemanticSearchResult` contains:

| Field        | Description                                              |
|-------------|----------------------------------------------------------|
| `file`      | Path relative to the indexed directory                   |
| `line_start`| 1-indexed first line of the matching chunk               |
| `line_end`  | 1-indexed last line of the matching chunk                |
| `content`   | The full chunk text (function body, paragraph, etc.)     |
| `score`     | Relevance score (higher = more relevant after reranking) |
| `symbol`    | Function/struct/class name, if extracted from AST        |
| `language`  | Programming language, if detected                        |

---

## Step 6: Filter results

Use `file_filter` globs to restrict search to specific paths:

```rust,ignore
// Only search Rust files in the auth module:
let auth_results = vfs.semantic_search("credential validation", SemanticSearchOptions {
    top_k: Some(3),
    min_score: Some(0.5),
    file_filter: vec!["src/auth/**/*.rs".into()],
    rerank: Some(true),
}).await?;
```

Use `min_score` to exclude low-confidence results. The appropriate threshold
depends on your use case — start with `None` and observe the score distribution,
then set a threshold that filters noise.

---

## Step 7: Incremental updates

After the initial index, the file watcher keeps it up to date automatically.
When you save a file, the watcher detects the change, re-chunks the file,
re-embeds it, and updates the vector store. No manual re-indexing needed.

To force a full re-index (e.g. after a large merge):

```rust,ignore
let handle = vfs.index("src", IndexOptions {
    force: true,  // ignore cache, re-index everything
    ..Default::default()
}).await?;
```

---

## Direct API: complete example

```rust,ignore
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{IndexOptions, IndexStatus, SemanticSearchOptions};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create VFS with local filesystem access
    let vfs = LocalProvider::new(PathBuf::from("."))?;

    // 2. Start indexing
    let handle = vfs.index("src", IndexOptions::default()).await?;

    // 3. Wait for completion
    loop {
        match vfs.index_status(&handle.index_id).await? {
            IndexStatus::Ready(_) => break,
            IndexStatus::Failed(e) => return Err(e.into()),
            _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }

    // 4. Search by meaning
    let results = vfs.semantic_search(
        "database connection pooling",
        SemanticSearchOptions::default(),
    ).await?;

    for r in &results {
        println!("{} (lines {}-{}): {:.3}", r.file, r.line_start, r.line_end, r.score);
    }

    Ok(())
}
```

---

## Built-in agent tools: semantic search as a tool call

The VFS tools module automatically generates `index`, `index_status`, and `semantic_search` tools when the provider has the `INDEX` and `SEMANTIC_SEARCH` capabilities. You don't write tool wrappers — they're built in.

### How it works

`vfs_tools()` inspects the provider's capabilities and emits only the tools the provider supports:

```rust,ignore
use std::sync::Arc;
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::{vfs_tools, OutputFormat};
use std::path::PathBuf;

let vfs = Arc::new(LocalProvider::new(PathBuf::from("."))?);

// This returns tools for ls, read, write, grep, glob, find,
// AND index, index_status, semantic_search (because LocalProvider
// with semantic-search feature has those capabilities).
let tools = vfs_tools(Arc::clone(&vfs) as Arc<_>, OutputFormat::Plain);

for tool in &tools {
    println!("  {}: {}", tool.name(), tool.description());
}
```

### Giving the agent semantic search

Pass the VFS tools to an agent — the LLM sees `semantic_search` as a callable tool alongside `read_file`, `grep`, etc.:

```rust,ignore
use std::sync::Arc;
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;
use synwire_core::vfs::{vfs_tools, OutputFormat};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let vfs = Arc::new(
        LocalProvider::new(PathBuf::from("."))?
    );

    // Built-in tools include semantic_search, index, index_status,
    // plus all file operations (read, write, ls, grep, glob, etc.)
    let tools = vfs_tools(Arc::clone(&vfs) as Arc<_>, OutputFormat::Plain);

    let agent = Agent::new("code-assistant", "claude-opus-4-6")
        .system_prompt(
            "You are a code assistant with access to the local filesystem.\n\
             You have both grep (exact text search) and semantic_search \
             (meaning-based search).\n\n\
             IMPORTANT: Before using semantic_search, you must call the \
             `index` tool to build the index, then poll `index_status` \
             until it reports Ready.\n\n\
             Use grep for known patterns (function names, error strings).\n\
             Use semantic_search for conceptual queries ('how are errors \
             handled?', 'authentication flow')."
        )
        .tools(tools)  // all VFS tools including semantic_search
        .max_turns(30);

    let runner = Runner::new(agent);
    let mut stream = runner
        .run(
            serde_json::json!("Find all the error handling patterns in this project"),
            RunnerConfig::default(),
        )
        .await?;

    while let Some(event) = stream.recv().await {
        match event {
            AgentEvent::TextDelta { content } => print!("{content}"),
            AgentEvent::ToolCallStart { name, .. } => {
                eprintln!("\n[calling {name}]");
            }
            _ => {}
        }
    }

    Ok(())
}
```

The agent autonomously decides the workflow:
1. Calls `index("src", {})` to start indexing
2. Polls `index_status` until `Ready`
3. Calls `semantic_search("error handling patterns", { top_k: 10 })`
4. Reads the results, possibly calls `read_file` on interesting hits for full context
5. Synthesises an answer

You write zero tool wrappers — `vfs_tools` handles everything.

### Combining grep and semantic search

The built-in tools let the agent choose the right search for each sub-query:

```rust,ignore
let agent = Agent::new("researcher", "claude-opus-4-6")
    .system_prompt(
        "You have two search tools:\n\
         - `grep`: fast exact text/regex search — use for known identifiers\n\
         - `semantic_search`: meaning-based search — use for conceptual queries\n\n\
         Strategy: start with semantic_search for broad understanding, then \
         use grep to find exact call sites of specific symbols you discover."
    )
    .tools(vfs_tools(vfs, OutputFormat::Plain))
    .max_turns(30);
```

The agent might:
1. `semantic_search("authentication and authorization")` → finds `fn verify_token` in `auth.rs`
2. `grep("verify_token")` → finds all 14 call sites across the codebase
3. `read_file("src/middleware/auth_middleware.rs")` → reads the main consumer

This grep-then-semantic or semantic-then-grep pattern is natural for LLMs and requires no special orchestration.

---

## StateGraph node: semantic search in a pipeline

For structured multi-step workflows, embed semantic search as a node in a `StateGraph`. This is useful when search results feed into a downstream processing step — summarisation, code generation, or report writing.

### Example: Research pipeline with semantic search

This graph indexes the codebase, searches for relevant code, and produces a summary:

```rust,ignore
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use synwire_derive::State;
use synwire_orchestrator::constants::END;
use synwire_orchestrator::error::GraphError;
use synwire_orchestrator::graph::StateGraph;

use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{IndexOptions, IndexStatus, SemanticSearchOptions};

/// Pipeline state flowing through three nodes.
#[derive(State, Debug, Clone, Default, Serialize, Deserialize)]
struct ResearchState {
    /// The conceptual query to research.
    #[reducer(last_value)]
    query: String,

    /// Project root path.
    #[reducer(last_value)]
    project_root: String,

    /// Raw search results (file:lines → content).
    #[reducer(topic)]
    search_hits: Vec<String>,

    /// Final summary produced by the summarise node.
    #[reducer(last_value)]
    summary: String,
}
```

#### Node 1: Index and search

This node handles both indexing and searching — it's a pure Rust function, no agent needed:

```rust,ignore
/// Index the codebase and run a semantic search.
async fn search_node(mut state: ResearchState) -> Result<ResearchState, GraphError> {
    let vfs = LocalProvider::new(state.project_root.clone().into())
        .map_err(|e| GraphError::NodeError { message: e.to_string() })?;

    // Start indexing.
    let handle = vfs.index("src", IndexOptions::default()).await
        .map_err(|e| GraphError::NodeError { message: e.to_string() })?;

    // Wait for completion.
    loop {
        match vfs.index_status(&handle.index_id).await
            .map_err(|e| GraphError::NodeError { message: e.to_string() })?
        {
            IndexStatus::Ready(_) => break,
            IndexStatus::Failed(e) => {
                return Err(GraphError::NodeError { message: e.to_string() });
            }
            _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }

    // Search by meaning.
    let results = vfs.semantic_search(&state.query, SemanticSearchOptions {
        top_k: Some(10),
        rerank: Some(true),
        ..Default::default()
    }).await
        .map_err(|e| GraphError::NodeError { message: e.to_string() })?;

    // Record hits as "file:start-end → content" strings.
    for r in &results {
        let hit = format!(
            "{}:{}-{} [score={:.3}{}]\n{}",
            r.file,
            r.line_start,
            r.line_end,
            r.score,
            r.symbol.as_deref().map_or(String::new(), |s| format!(", symbol={s}")),
            r.content,
        );
        state.search_hits.push(hit);
    }

    Ok(state)
}
```

#### Node 2: Summarise

An LLM agent reads the search hits and produces a structured summary:

```rust,ignore
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;

/// Summarise the search results into a concise report.
async fn summarise_node(mut state: ResearchState) -> Result<ResearchState, GraphError> {
    let system = "You receive semantic search results from a codebase and produce \
                  a concise technical summary. Group findings by theme. Include \
                  file paths and line numbers for every claim.";

    let prompt = format!(
        "Query: {}\n\nSearch results ({} hits):\n\n{}",
        state.query,
        state.search_hits.len(),
        state.search_hits.join("\n---\n"),
    );

    let agent = Agent::new("summariser", "claude-opus-4-6")
        .system_prompt(system)
        .max_turns(1);

    let runner = Runner::new(agent);
    let mut stream = runner
        .run(serde_json::json!(prompt), RunnerConfig::default())
        .await
        .map_err(|e| GraphError::NodeError { message: e.to_string() })?;

    let mut summary = String::new();
    while let Some(event) = stream.recv().await {
        match event {
            AgentEvent::TextDelta { content } => summary.push_str(&content),
            AgentEvent::Error { message } => {
                return Err(GraphError::NodeError { message });
            }
            _ => {}
        }
    }

    state.summary = summary;
    Ok(state)
}
```

#### Assembling the graph

```rust,ignore
let mut graph = StateGraph::<ResearchState>::new();

graph.add_node("search",    Box::new(|s| Box::pin(search_node(s))))?;
graph.add_node("summarise", Box::new(|s| Box::pin(summarise_node(s))))?;

graph
    .set_entry_point("search")
    .add_edge("search", "summarise")
    .add_edge("summarise", END);

let pipeline = graph.compile()?;

// Run the pipeline.
let result = pipeline.invoke(ResearchState {
    query: "How does error propagation work across module boundaries?".into(),
    project_root: "/path/to/project".into(),
    ..Default::default()
}).await?;

println!("{}", result.summary);
```

### When to use each approach

| Approach | Use when | Example |
|---|---|---|
| **Direct API** | You control the flow yourself; no agent involved | CLI tools, scripts, tests |
| **Built-in agent tool** | The agent decides when to search; search is one of many actions | Coding agents, Q&A bots, interactive assistants |
| **StateGraph node** | Search is a fixed stage in a multi-step pipeline | Research pipelines, batch analysis, report generation |

The built-in tool approach is the most common — it gives the agent maximum autonomy while requiring zero wrapper code. The graph approach is best when search must happen at a specific point in a deterministic workflow.

---

## Wrapping the pipeline as an agent tool

Following the pattern from [Tutorial 8](./08-research-coding-pipeline.md), you can wrap the entire search → summarise graph as a `StructuredTool` and give it to an agent:

```rust,ignore
use std::sync::Arc;
use synwire_core::tools::{StructuredTool, ToolOutput, ToolSchema};

fn semantic_research_tool(project_root: String) -> Result<StructuredTool, synwire_core::error::SynwireError> {
    let graph = Arc::new(build_research_graph()?);
    let root = project_root.clone();

    StructuredTool::builder()
        .name("semantic_research")
        .description(
            "Searches the codebase by meaning and returns a summarised report. \
             Use for broad conceptual queries about how the codebase works."
        )
        .schema(ToolSchema {
            name: "semantic_research".into(),
            description: "Semantic codebase research".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Conceptual query about the codebase"
                    }
                },
                "required": ["query"]
            }),
        })
        .func(move |input| {
            let graph = Arc::clone(&graph);
            let project_root = root.clone();
            Box::pin(async move {
                let query = input["query"].as_str().unwrap_or("overview").to_owned();
                let result = graph.invoke(ResearchState {
                    query,
                    project_root,
                    ..Default::default()
                }).await.map_err(|e| synwire_core::error::SynwireError::Tool(
                    synwire_core::error::ToolError::InvocationFailed {
                        message: format!("research pipeline failed: {e}"),
                    },
                ))?;
                Ok(ToolOutput {
                    content: result.summary,
                    ..Default::default()
                })
            })
        })
        .build()
}
```

Now the agent has three levels of search capability:

| Tool | Granularity | When the agent uses it |
|---|---|---|
| `grep` | Exact text/regex match | Known identifiers, error strings |
| `semantic_search` | Raw vector search results | Focused conceptual queries |
| `semantic_research` | Summarised research report | Broad "how does X work?" questions |

The agent picks the right tool for the job — grep for precision, semantic_search for discovery, semantic_research for understanding.

---

## What you learned

- The `semantic-search` feature flag enables local semantic search on `LocalProvider`
- `index()` starts background indexing and returns an `IndexHandle` immediately
- `index_status()` polls progress until the index is ready
- `semantic_search()` finds code by meaning, with optional filtering and reranking
- `vfs_tools()` automatically generates agent tools for all VFS capabilities — including `index`, `index_status`, and `semantic_search`
- `StateGraph` nodes can run the index/search pipeline as a deterministic stage
- The graph can be wrapped as a `StructuredTool` for agent-driven research

---

## See also

- [Semantic Search Architecture](../explanation/semantic-search-architecture.md) — the four-stage pipeline in depth
- [Semantic Search How-To](../how-to/semantic-search.md) — task-focused recipes
- [synwire-chunker](../explanation/synwire-chunker.md) — AST-aware code chunking
- [synwire-embeddings-local](../explanation/synwire-embeddings-local.md) — local ONNX embedding models
- [synwire-vectorstore-lancedb](../explanation/synwire-vectorstore-lancedb.md) — LanceDB vector storage
- [synwire-index](../explanation/synwire-index.md) — indexing pipeline lifecycle
- [Tutorial 8: Deep Research + Coding Agent](./08-research-coding-pipeline.md) — graph-as-tool composition pattern
- [Tutorial 7: Building a Coding Agent](./07-coding-agent.md) — combine semantic search with tool use

> **Background**: [RAG techniques](https://www.promptingguide.ai/techniques/rag) — semantic search is the retrieval step. The summarise node or the agent's reasoning is the generation step. The two-stage retrieve-then-rerank pipeline improves relevance without sacrificing speed.
