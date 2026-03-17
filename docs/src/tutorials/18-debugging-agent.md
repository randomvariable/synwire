# Building a Debugging Agent

**Time**: ~90 minutes
**Prerequisites**: Rust 1.85+, completion of tutorials [14](./14-fault-localization.md), [15](./15-dataflow-analysis.md), and [16](./16-code-analysis-tools.md)

> **Background**: [ReAct Agents](https://www.promptingguide.ai/techniques/react) -- the reason-then-act loop that debugging agents use. Each observation (search result, coverage data, variable trace) feeds back into the next reasoning step.

This tutorial builds an end-to-end debugging agent that accepts a bug report and produces a root cause analysis with a proposed fix. It combines semantic search, SBFL fault localization, dataflow analysis, LSP tools, and file operations into a single agent.

---

## What you are building

A binary that:

1. Accepts a bug report as input
2. Uses semantic search to find relevant code
3. Uses SBFL to identify suspicious files from test coverage
4. Uses dataflow analysis to trace variable origins
5. Uses LSP tools for precise type information
6. Produces a root cause analysis and suggests a fix

---

## Step 1: Dependencies

```toml
[dependencies]
synwire-agent = { version = "0.1", features = ["semantic-search"] }
synwire-core = { version = "0.1" }
synwire-lsp = { version = "0.1" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

---

## Step 2: Set up the VFS and tools

The agent needs access to file operations, semantic search, and analysis tools. Start by creating the VFS and collecting all tools:

```rust,ignore
use std::sync::Arc;
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::{vfs_tools, OutputFormat};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = PathBuf::from(".");
    let vfs = Arc::new(LocalProvider::new(project_root)?);

    // VFS tools: fs.read, fs.write, fs.edit, fs.grep, fs.glob, fs.tree,
    // code.skeleton, index.run, index.status, index.search
    let tools = vfs_tools(Arc::clone(&vfs) as Arc<_>, OutputFormat::Plain);

    // The MCP server adds code.fault_localize, code.trace_dataflow, and
    // code.trace_callers on top of VFS tools. When using the Rust API
    // directly, we construct them as StructuredTool instances.

    Ok(())
}
```

---

## Step 3: Create analysis tools

Wrap the SBFL, dataflow, and call graph modules as agent tools:

```rust,ignore
use synwire_core::tools::{StructuredTool, ToolOutput, ToolSchema};
use synwire_agent::sbfl::{CoverageRecord, SbflRanker, fuse_sbfl_semantic};
use synwire_agent::dataflow::DataflowTracer;
use synwire_agent::call_graph::DynamicCallGraph;

fn fault_localize_tool() -> StructuredTool {
    StructuredTool::builder()
        .name("code.fault_localize")
        .description(
            "Rank source files by fault likelihood using SBFL/Ochiai. \
             Provide coverage data as an array of {file, line, ef, nf, np} objects."
        )
        .schema(ToolSchema {
            name: "code.fault_localize".into(),
            description: "SBFL fault localization".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "coverage": {
                        "type": "array",
                        "items": { "type": "object" }
                    }
                },
                "required": ["coverage"]
            }),
        })
        .func(|input| Box::pin(async move {
            let coverage: Vec<CoverageRecord> = input["coverage"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| Some(CoverageRecord {
                    file: v["file"].as_str()?.to_owned(),
                    line: v["line"].as_u64()? as u32,
                    ef: v["ef"].as_u64()? as u32,
                    nf: v["nf"].as_u64()? as u32,
                    np: v["np"].as_u64()? as u32,
                }))
                .collect();

            let ranker = SbflRanker::new(coverage);
            let ranked = ranker.rank_files();

            let output = ranked
                .iter()
                .map(|(f, s)| format!("{f}: {s:.3}"))
                .collect::<Vec<_>>()
                .join("\n");

            Ok(ToolOutput { content: output, ..Default::default() })
        }))
        .build()
        .expect("valid tool")
}

fn dataflow_trace_tool(vfs: Arc<dyn synwire_core::vfs::protocol::Vfs>) -> StructuredTool {
    StructuredTool::builder()
        .name("code.trace_dataflow")
        .description(
            "Trace a variable's assignments backward through a source file. \
             Returns definition and assignment sites."
        )
        .schema(ToolSchema {
            name: "code.trace_dataflow".into(),
            description: "Variable dataflow tracing".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string" },
                    "variable": { "type": "string" },
                    "max_hops": { "type": "integer" }
                },
                "required": ["file", "variable"]
            }),
        })
        .func(move |input| {
            let vfs = Arc::clone(&vfs);
            Box::pin(async move {
                let file = input["file"].as_str().unwrap_or("");
                let variable = input["variable"].as_str().unwrap_or("");
                let max_hops = input["max_hops"].as_u64().unwrap_or(10) as u32;

                let content = vfs.read(file).await.map_err(|e|
                    synwire_core::error::SynwireError::Tool(
                        synwire_core::error::ToolError::InvocationFailed {
                            message: e.to_string(),
                        },
                    )
                )?;

                let tracer = DataflowTracer::new(max_hops);
                let hops = tracer.trace(&content, variable, file);

                let output = hops
                    .iter()
                    .map(|h| format!(
                        "[{}] {}:{} -- {}",
                        h.origin.kind, h.origin.file, h.origin.line, h.origin.snippet,
                    ))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(ToolOutput { content: output, ..Default::default() })
            })
        })
        .build()
        .expect("valid tool")
}
```

---

## Step 4: Configure the agent

The system prompt guides the agent through a structured debugging workflow:

```rust,ignore
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;

let mut all_tools = tools; // VFS tools from Step 2
all_tools.push(Box::new(fault_localize_tool()));
all_tools.push(Box::new(dataflow_trace_tool(Arc::clone(&vfs) as Arc<_>)));

let agent = Agent::new("debugger", "claude-opus-4-6")
    .system_prompt(
        "You are an expert debugging agent. Follow this structured process:\n\n\
         ## Phase 1: Understand the bug\n\
         - Parse the bug report for symptoms, expected vs actual behaviour\n\
         - Identify key terms, error messages, and affected features\n\n\
         ## Phase 2: Locate relevant code\n\
         - Call `index.run` then `index.search` with the bug description\n\
         - Use `fs.grep` for specific error messages or identifiers\n\
         - Use `fs.tree` to understand project structure if needed\n\n\
         ## Phase 3: Analyse fault likelihood\n\
         - If test coverage data is available, call `code.fault_localize`\n\
         - Read the top-ranked files\n\n\
         ## Phase 4: Trace data flow\n\
         - For suspicious variables, call `code.trace_dataflow`\n\
         - Follow the chain of assignments to find the origin\n\n\
         ## Phase 5: Report\n\
         - State the root cause with file paths and line numbers\n\
         - Explain the causal chain from origin to symptom\n\
         - Suggest a specific fix with code changes\n\n\
         Be methodical. Show your reasoning at each phase."
    )
    .tools(all_tools)
    .max_turns(40);
```

---

## Step 5: Run the agent

```rust,ignore
let runner = Runner::new(agent);
let config = RunnerConfig::default();

let bug_report = serde_json::json!(
    "Bug #4521: When a user submits a form with special characters in the \
     'name' field (e.g. O'Brien), the server returns a 500 error. \
     The error log shows: 'SqliteError: unrecognized token near O'. \
     This started after the recent refactor of the user service."
);

let mut rx = runner.run(bug_report, config).await?;

while let Some(event) = rx.recv().await {
    match event {
        AgentEvent::TextDelta { content } => print!("{content}"),
        AgentEvent::ToolCallStart { name, .. } => {
            eprintln!("\n--- [{name}] ---");
        }
        AgentEvent::ToolCallEnd { name, .. } => {
            eprintln!("--- [/{name}] ---\n");
        }
        AgentEvent::TurnComplete { reason } => {
            println!("\n\n[Agent finished: {reason:?}]");
        }
        AgentEvent::Error { message } => {
            eprintln!("Error: {message}");
        }
        _ => {}
    }
}
```

---

## Expected agent behaviour

For the SQL injection bug above, the agent would typically:

1. **Semantic search** for "SQL query construction" and "user input sanitisation"
2. **`fs.grep`** for the error string `unrecognized token`
3. **Read** the user service files found by search
4. **Dataflow trace** the `name` variable to find where it enters the SQL query
5. **Report** that user input is interpolated directly into a SQL string without escaping, and suggest using parameterised queries

---

## Step 6: Add LSP for precision

When `synwire-lsp` is available, add LSP tools for type-aware investigation:

```rust,ignore
use synwire_lsp::{client::LspClient, config::LspServerConfig, tools::lsp_tools};

let lsp_config = LspServerConfig::new("rust-analyzer");
let lsp_client = LspClient::start(&lsp_config).await?;
lsp_client.initialize().await?;

let lsp_tool_set = lsp_tools(Arc::new(lsp_client));

// Add LSP tools to the agent's tool set
all_tools.extend(lsp_tool_set);
```

With LSP tools, the agent can:
- `lsp.hover` to check the type of a variable (is `name` a `&str` or a sanitised `SafeString`?)
- `lsp.goto_definition` to find the exact function that builds the SQL query
- `lsp.references` to find all call sites that pass unsanitised input

---

## Putting it all together

The complete `main.rs`:

```rust,ignore
use std::sync::Arc;
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;
use synwire_core::vfs::{vfs_tools, OutputFormat};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vfs = Arc::new(LocalProvider::new(PathBuf::from("."))?);

    let mut tools = vfs_tools(Arc::clone(&vfs) as Arc<_>, OutputFormat::Plain);
    tools.push(Box::new(fault_localize_tool()));
    tools.push(Box::new(dataflow_trace_tool(Arc::clone(&vfs) as Arc<_>)));

    let agent = Agent::new("debugger", "claude-opus-4-6")
        .system_prompt("...")  // system prompt from Step 4
        .tools(tools)
        .max_turns(40);

    let runner = Runner::new(agent);
    let bug = std::env::args().nth(1).unwrap_or_else(|| {
        "Describe the bug here".to_string()
    });

    let mut rx = runner
        .run(serde_json::json!(bug), RunnerConfig::default())
        .await?;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::TextDelta { content } => print!("{content}"),
            AgentEvent::ToolCallStart { name, .. } => {
                eprintln!("\n--- [{name}] ---");
            }
            AgentEvent::TurnComplete { reason } => {
                println!("\n[{reason:?}]");
            }
            _ => {}
        }
    }

    Ok(())
}
```

Run it:

```bash
cargo run -- "Bug #4521: form submission with special characters causes 500 error"
```

---

## What you learned

- A debugging agent combines semantic search, SBFL, dataflow tracing, and file operations
- The system prompt structures the agent's investigation into phases
- Analysis modules from `synwire-agent` can be wrapped as `StructuredTool` instances
- LSP tools add type-aware precision to the investigation
- The agent autonomously decides which tools to call and in what order

---

## See also

- [Tutorial 7: Building a Coding Agent](./07-coding-agent.md) -- general coding agent pattern
- [Tutorial 14: Fault Localization](./14-fault-localization.md) -- SBFL in depth
- [Tutorial 15: Dataflow Analysis](./15-dataflow-analysis.md) -- variable tracing
- [Tutorial 16: Code Analysis Tools](./16-code-analysis-tools.md) -- call graphs and combined analysis
- [Tutorial 17: Advanced MCP Setup](./17-advanced-mcp-setup.md) -- all these tools via MCP
- [How-To: Approval Gates](../how-to/approval-gates.md) -- requiring human approval before the agent applies fixes
