# Dataflow Analysis

**Time**: ~30 minutes
**Prerequisites**: Rust 1.85+, completion of [Your First Agent](./01-first-agent.md)

> **Background**: Dataflow analysis traces how values propagate through code -- where a variable is defined, where it is reassigned, and which expressions contribute to its current value. This is essential for understanding *why* a variable holds an unexpected value.

This tutorial shows how to use the `DataflowTracer` to trace variable origins in source code, and how to integrate it into an agent workflow for automated debugging.

---

## What the tracer does

`DataflowTracer` performs heuristic backward slicing: given a variable name and source text, it finds assignment sites (`x = ...`) and definition sites (`let x = ...`) by pattern matching. Each result is a `DataflowHop` containing the file, line, code snippet, and origin kind.

This is a lightweight analysis -- it does not require compilation or a running language server. For precise type-aware tracing, combine it with LSP tools (see [How-To: LSP Integration](../how-to/lsp-integration.md)).

---

## Step 1: Add dependencies

```toml
[dependencies]
synwire-agent = { version = "0.1" }
```

---

## Step 2: Trace a variable in Rust source

```rust,ignore
use synwire_agent::dataflow::DataflowTracer;

fn main() {
    let source = r#"
fn process_request(input: &str) -> Result<Response, Error> {
    let config = load_config()?;
    let timeout = config.timeout_ms;
    let client = HttpClient::new(timeout);
    let response = client.get(input)?;
    let status = response.status();
    status = override_status(status);
    Ok(Response::new(status))
}
"#;

    let tracer = DataflowTracer::new(10);
    let hops = tracer.trace(source, "status", "src/handler.rs");

    for hop in &hops {
        println!(
            "[{}] {}:{} -- {}",
            hop.origin.kind,
            hop.origin.file,
            hop.origin.line,
            hop.origin.snippet,
        );
    }
}
```

Output:

```text
[definition] src/handler.rs:6 -- let status = response.status();
[assignment] src/handler.rs:7 -- status = override_status(status);
```

The tracer found two sites: the initial `let` binding and a subsequent reassignment. An agent investigating an unexpected status value now knows to look at both `response.status()` and `override_status`.

---

## Step 3: Control trace depth

The `max_hops` parameter limits how many origin sites are returned. Use a small value (2--3) for focused traces, or a larger value (10+) for thorough analysis of heavily-reassigned variables:

```rust,ignore
// Only find the first 2 assignment sites
let tracer = DataflowTracer::new(2);
let hops = tracer.trace(source, "x", "lib.rs");
assert!(hops.len() <= 2);
```

---

## Step 4: Trace variables in other languages

The tracer works on any language that uses `=` for assignment and `let` for bindings. For languages like Python or JavaScript, the assignment patterns still match:

```rust,ignore
let python_source = r#"
def calculate_total(items):
    total = 0
    for item in items:
        price = item.get_price()
        total = total + price
    total = apply_discount(total)
    return total
"#;

let tracer = DataflowTracer::new(10);
let hops = tracer.trace(python_source, "total", "cart.py");

for hop in &hops {
    println!("{}: line {} -- {}", hop.origin.kind, hop.origin.line, hop.origin.snippet);
}
// definition: line 2 -- total = 0
// assignment: line 5 -- total = total + price
// assignment: line 6 -- total = apply_discount(total)
```

---

## Step 5: Use as an MCP tool

The `code.trace_dataflow` MCP tool wraps `DataflowTracer`. An agent calls it with a file path and variable name:

```json
{
  "tool": "code.trace_dataflow",
  "arguments": {
    "file": "src/handler.rs",
    "variable": "status",
    "max_hops": 10
  }
}
```

The server reads the file, runs the tracer, and returns the hops as structured text.

---

## Step 6: Integrate into an agent workflow

Combine dataflow tracing with file reading and semantic search to build an automated variable investigator:

```rust,ignore
use std::sync::Arc;
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;

let agent = Agent::new("tracer", "claude-opus-4-6")
    .system_prompt(
        "You investigate unexpected variable values. Your workflow:\n\
         1. Read the file containing the variable\n\
         2. Call code.trace_dataflow to find all assignment sites\n\
         3. For each assignment, use semantic_search or grep to find the \
            called functions\n\
         4. Explain the full data flow from origin to the point of failure"
    )
    .tools(tools)  // VFS tools + code.trace_dataflow
    .max_turns(15);

let runner = Runner::new(agent);
let mut rx = runner
    .run(
        serde_json::json!("The variable `timeout` in src/client.rs has value 0 -- trace where it comes from"),
        RunnerConfig::default(),
    )
    .await?;

while let Some(event) = rx.recv().await {
    match event {
        AgentEvent::TextDelta { content } => print!("{content}"),
        AgentEvent::ToolCallStart { name, .. } => eprintln!("\n[calling {name}]"),
        _ => {}
    }
}
```

The agent reads the file, calls `code.trace_dataflow` to find assignment sites for `timeout`, then reads each called function to explain the full data flow.

---

## Limitations

- The tracer uses text pattern matching, not a full AST. It may produce false positives for variables whose names appear in comments or strings.
- It does not trace across function boundaries. Use LSP `goto_definition` to follow values through call chains.
- For languages without `let` or `=` assignment syntax, results may be incomplete.

---

## What you learned

- `DataflowTracer` finds where a variable is defined and modified using heuristic pattern matching
- `max_hops` controls how many origin sites are returned
- The tracer works across languages that use standard assignment syntax
- The `code.trace_dataflow` MCP tool makes this available to agents
- Combine with LSP tools for cross-function tracing

---

## See also

- [Tutorial 14: Fault Localization](./14-fault-localization.md) -- ranking files by suspiciousness
- [Tutorial 16: Code Analysis Tools](./16-code-analysis-tools.md) -- combining dataflow with call graphs
- [How-To: LSP Integration](../how-to/lsp-integration.md) -- precise type-aware goto-definition
- [Tutorial 18: Building a Debugging Agent](./18-debugging-agent.md) -- full debugging workflow
