# Code Analysis Tools

**Time**: ~1 hour
**Prerequisites**: Rust 1.85+, completion of [Tutorial 9: Semantic Search](./09-semantic-search.md)

> **Background**: [Code Intelligence](https://www.promptingguide.ai/agents/components) -- understanding code structure through call graphs, data flow, and semantic search enables agents to reason about codebases the way developers do.

This tutorial shows how to combine Synwire's analysis tools -- call graphs, semantic search, and dataflow tracing -- to build an agent that investigates bug reports by understanding code structure.

---

## The `DynamicCallGraph` API

`DynamicCallGraph` is an incrementally-built directed graph. Each edge represents a caller-to-callee relationship discovered via LSP goto-definition or static analysis. The graph supports three key queries:

```rust,ignore
use synwire_agent::call_graph::DynamicCallGraph;

let mut graph = DynamicCallGraph::new();

// Build the graph edge by edge (typically populated by LSP results)
graph.add_edge("main", "parse_config");
graph.add_edge("main", "run_server");
graph.add_edge("run_server", "handle_request");
graph.add_edge("handle_request", "validate_auth");
graph.add_edge("handle_request", "execute_query");
graph.add_edge("execute_query", "db_connect");

// Query 1: What does handle_request call?
let callees = graph.callees("handle_request");
println!("handle_request calls: {:?}", callees);
// ["validate_auth", "execute_query"]

// Query 2: Who calls execute_query?
let callers = graph.callers("execute_query");
println!("execute_query called by: {:?}", callers);
// ["handle_request"]

// Query 3: Are there cycles?
println!("Has cycles: {}", graph.has_cycle());
// false
```

---

## Step 1: Build a call graph from LSP

In practice, the call graph is populated by following LSP goto-definition results. The MCP server's `code.trace_callers` tool does this automatically, but you can also build it programmatically:

```rust,ignore
use synwire_agent::call_graph::{CallNode, DynamicCallGraph};

/// Build a call graph by following goto-definition for each function call.
async fn build_call_graph(
    lsp: &synwire_lsp::client::LspClient,
    entry_file: &str,
) -> DynamicCallGraph {
    let mut graph = DynamicCallGraph::new();

    // Get all symbols in the entry file
    let symbols = lsp.document_symbols(entry_file).await.unwrap_or_default();

    for symbol in &symbols {
        // For each function, find what it calls via references
        let refs = lsp.references(entry_file, symbol.line, symbol.column).await;
        if let Ok(locations) = refs {
            for loc in &locations {
                graph.add_edge(&symbol.name, &loc.symbol_name);
            }
        }
    }

    graph
}
```

---

## Step 2: Detect dependency cycles

Circular dependencies often indicate architectural problems. The `has_cycle` method uses depth-first search to detect them:

```rust,ignore
let mut graph = DynamicCallGraph::new();
graph.add_edge("module_a::init", "module_b::setup");
graph.add_edge("module_b::setup", "module_c::configure");
graph.add_edge("module_c::configure", "module_a::init"); // cycle!

if graph.has_cycle() {
    println!("Circular dependency detected!");
    // An agent could report the cycle and suggest refactoring
}
```

---

## Step 3: Combine call graph with semantic search

Use semantic search to find relevant code, then expand understanding with the call graph:

```rust,ignore
use synwire_core::vfs::types::SemanticSearchOptions;

// Step 1: Semantic search finds entry points
let results = vfs.semantic_search(
    "authentication token validation",
    SemanticSearchOptions { top_k: Some(5), ..Default::default() },
).await?;

// Step 2: For each result, query the call graph to find callers and callees
let mut graph = DynamicCallGraph::new();
for result in &results {
    if let Some(ref symbol) = result.symbol {
        // Use LSP or MCP code.trace_callers to populate edges
        let callees = get_callees_from_lsp(symbol).await;
        for callee in &callees {
            graph.add_edge(symbol, callee);
        }
    }
}

// Step 3: Find all callers of the token validator
let callers = graph.callers("validate_token");
println!("validate_token is called by: {:?}", callers);
```

This pattern -- semantic search for discovery, call graph for structure -- is how agents build a mental model of unfamiliar code.

---

## Step 4: Use analysis tools via MCP

The MCP server exposes three analysis tools that agents can call:

| Tool | Namespace | Purpose |
|------|-----------|---------|
| `code.trace_callers` | `code` | Query callers/callees of a symbol |
| `code.trace_dataflow` | `code` | Trace variable assignments backward |
| `code.fault_localize` | `code` | Rank files by test failure correlation |

An agent investigating a bug typically uses them in sequence:

```json
// 1. Find relevant code
{"tool": "index.search", "arguments": {"query": "payment processing error"}}

// 2. Understand the call chain
{"tool": "code.trace_callers", "arguments": {"symbol": "process_payment", "direction": "both"}}

// 3. Trace the problematic variable
{"tool": "code.trace_dataflow", "arguments": {"file": "src/payment.rs", "variable": "amount"}}

// 4. Rank files by fault likelihood (if tests are failing)
{"tool": "code.fault_localize", "arguments": {"coverage": [...]}}
```

---

## Step 5: Build a bug investigation agent

Combine all analysis tools into a single agent that can investigate a bug report end-to-end:

```rust,ignore
use std::sync::Arc;
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;

let agent = Agent::new("investigator", "claude-opus-4-6")
    .system_prompt(
        "You are a code investigation agent. Given a bug report, follow this process:\n\n\
         1. Use index.search to find code related to the bug description\n\
         2. Use code.trace_callers to understand what calls what\n\
         3. Use code.trace_dataflow on suspicious variables\n\
         4. Use read to examine specific functions in detail\n\
         5. Synthesize your findings into a root cause analysis\n\n\
         Always explain your reasoning at each step."
    )
    .tools(tools)  // VFS + index.search + code.trace_callers + code.trace_dataflow
    .max_turns(30);

let runner = Runner::new(agent);
let mut rx = runner
    .run(
        serde_json::json!(
            "Bug #1234: Users report that discount codes are applied twice \
             when checking out with multiple items. The total shown is lower \
             than expected."
        ),
        RunnerConfig::default(),
    )
    .await?;

while let Some(event) = rx.recv().await {
    match event {
        AgentEvent::TextDelta { content } => print!("{content}"),
        AgentEvent::ToolCallStart { name, .. } => {
            eprintln!("\n[{name}]");
        }
        _ => {}
    }
}
```

The agent might:
1. Search for "discount code application" via `index.search`
2. Find `apply_discount` in `src/checkout.rs`
3. Query `code.trace_callers` for callers of `apply_discount` -- discovers it is called from both `process_item` and `finalize_cart`
4. Trace `discount_amount` via `code.trace_dataflow` -- finds it is accumulated without resetting
5. Report that the discount is applied per-item *and* per-cart, causing double application

---

## What you learned

- `DynamicCallGraph` stores caller/callee relationships and detects cycles
- Call graph queries reveal code structure that text search cannot
- Combining semantic search (find relevant code) with call graphs (understand structure) and dataflow (trace values) gives agents comprehensive code understanding
- The MCP tools `code.trace_callers`, `code.trace_dataflow`, and `code.fault_localize` are available via `synwire-mcp-server`

---

## See also

- [Tutorial 14: Fault Localization](./14-fault-localization.md) -- SBFL ranking
- [Tutorial 15: Dataflow Analysis](./15-dataflow-analysis.md) -- tracing variable origins
- [Tutorial 9: Semantic Search](./09-semantic-search.md) -- meaning-based code search
- [Tutorial 18: Building a Debugging Agent](./18-debugging-agent.md) -- full end-to-end debugging agent
- [How-To: LSP Integration](../how-to/lsp-integration.md) -- using LSP for precise goto-definition
