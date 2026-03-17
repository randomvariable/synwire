# Fault Localization with SBFL

**Time**: ~45 minutes
**Prerequisites**: Rust 1.85+, completion of [Your First Agent](./01-first-agent.md), familiarity with test coverage concepts

> **Background**: [Spectrum-Based Fault Localization](https://en.wikipedia.org/wiki/Fault_localization#Spectrum-based_techniques) -- a family of techniques that rank code locations by how strongly they correlate with test failures.

This tutorial shows how to use Synwire's `SbflRanker` to identify suspicious files from test coverage data, then fuse those rankings with semantic search results to produce a combined fault-likelihood score.

---

## What SBFL does

When a test suite has failures, some source lines are covered by failing tests but not by passing tests. SBFL assigns a suspiciousness score to each line using a statistical formula. Synwire uses the **Ochiai coefficient**:

```text
score = ef / sqrt((ef + nf) * (ef + ep))
```

Where:
- `ef` = number of **failing** tests that cover this line
- `nf` = number of **passing** tests that cover this line
- `ep` = number of **passing** tests that do **not** cover this line

A score of 1.0 means the line is covered exclusively by failing tests. A score of 0.0 means no failing test touches it.

---

## Step 1: Add dependencies

```toml
[dependencies]
synwire-agent = { version = "0.1" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

---

## Step 2: Build coverage records

`CoverageRecord` holds per-line coverage data. In practice you would parse output from `cargo-llvm-cov`, `gcov`, or a DAP coverage session. Here we construct records directly:

```rust,ignore
use synwire_agent::sbfl::{CoverageRecord, SbflRanker};

fn example_coverage() -> Vec<CoverageRecord> {
    vec![
        // Line 42 of buggy.rs: hit by 8 failing tests, 2 passing, 0 passing miss it
        CoverageRecord { file: "src/buggy.rs".into(), line: 42, ef: 8, nf: 2, np: 0 },
        CoverageRecord { file: "src/buggy.rs".into(), line: 43, ef: 6, nf: 4, np: 0 },
        // clean.rs: never hit by failing tests
        CoverageRecord { file: "src/clean.rs".into(), line: 10, ef: 0, nf: 5, np: 5 },
        CoverageRecord { file: "src/clean.rs".into(), line: 11, ef: 0, nf: 3, np: 7 },
        // utils.rs: sometimes hit by failing tests but also by many passing tests
        CoverageRecord { file: "src/utils.rs".into(), line: 20, ef: 3, nf: 7, np: 0 },
    ]
}
```

---

## Step 3: Rank files by suspiciousness

`SbflRanker` computes the Ochiai score for every line, then ranks files by their maximum score:

```rust,ignore
fn main() {
    let records = example_coverage();
    let ranker = SbflRanker::new(records);
    let ranked = ranker.rank_files();

    println!("Files ranked by fault likelihood:");
    for (file, score) in &ranked {
        println!("  {file}: {score:.3}");
    }
    // Output:
    //   src/buggy.rs: 0.894
    //   src/utils.rs: 0.548
    //   src/clean.rs: 0.000
}
```

`src/buggy.rs` scores highest because line 42 is covered almost exclusively by failing tests.

---

## Step 4: Fuse with semantic search

SBFL tells you *where* failures concentrate. Semantic search tells you *what code is relevant* to a bug description. Fusing both signals produces better results than either alone.

```rust,ignore
use synwire_agent::sbfl::fuse_sbfl_semantic;

let sbfl_scores = ranker.rank_files();

// Semantic search results (from Tutorial 09) -- score = relevance to bug description
let semantic_scores = vec![
    ("src/utils.rs".into(), 0.85_f32),
    ("src/buggy.rs".into(), 0.60),
    ("src/handler.rs".into(), 0.45),
];

// Fuse with 70% weight on SBFL, 30% on semantic similarity
let fused = fuse_sbfl_semantic(&sbfl_scores, &semantic_scores, 0.7);

println!("\nFused ranking (SBFL 70% + semantic 30%):");
for (file, score) in &fused {
    println!("  {file}: {score:.3}");
}
```

Adjusting `sbfl_weight` lets you shift emphasis. Use higher SBFL weight (0.7--0.8) when coverage data is reliable. Use lower weight (0.3--0.5) when coverage is sparse but the bug description is precise.

---

## Step 5: Use as an MCP tool

The `code.fault_localize` MCP tool wraps this pipeline. When running `synwire-mcp-server`, an agent can call it directly:

```json
{
  "tool": "code.fault_localize",
  "arguments": {
    "coverage": [
      {"file": "src/buggy.rs", "line": 42, "ef": 8, "nf": 2, "np": 0},
      {"file": "src/clean.rs", "line": 10, "ef": 0, "nf": 5, "np": 5}
    ],
    "semantic_results": [
      {"file": "src/buggy.rs", "score": 0.6},
      {"file": "src/utils.rs", "score": 0.85}
    ],
    "sbfl_weight": 0.7
  }
}
```

The tool returns files ranked by combined score. The agent can then `read` the top-ranked files and investigate further.

---

## Step 6: Wire it into an agent

Give the agent both `code.fault_localize` and VFS tools so it can autonomously collect coverage, rank files, and read the suspicious code:

```rust,ignore
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};

let agent = Agent::new("debugger", "claude-opus-4-6")
    .system_prompt(
        "You are a debugging assistant. When given a failing test:\n\
         1. Run the test suite to collect coverage data\n\
         2. Call code.fault_localize with the coverage records\n\
         3. Read the top-ranked files\n\
         4. Identify the root cause and suggest a fix"
    )
    .tools(tools)  // VFS tools + code.fault_localize
    .max_turns(20);

let runner = Runner::new(agent);
let mut rx = runner
    .run(serde_json::json!("test_auth_token is failing -- find the bug"), RunnerConfig::default())
    .await?;
```

---

## What you learned

- The Ochiai coefficient ranks code lines by how strongly they correlate with test failures
- `SbflRanker` aggregates line-level scores to file-level rankings
- `fuse_sbfl_semantic` combines SBFL with semantic search for better fault localization
- The `code.fault_localize` MCP tool exposes this pipeline to agents

---

## See also

- [Tutorial 9: Semantic Search](./09-semantic-search.md) -- generating the semantic scores to fuse with SBFL
- [Tutorial 15: Dataflow Analysis](./15-dataflow-analysis.md) -- tracing how a variable reaches the faulty line
- [Tutorial 18: Building a Debugging Agent](./18-debugging-agent.md) -- end-to-end debugging workflow
- [How-To: DAP Integration](../how-to/dap-integration.md) -- collecting coverage via the debug adapter
