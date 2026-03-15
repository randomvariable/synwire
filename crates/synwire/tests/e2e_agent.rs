//! E2E test: `ReAct` agent with tool calling.
//!
//! Requires a running Ollama instance. Run with `cargo nextest run --run-ignored all`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_agent_tool_calling() {
    // Stub: when agent infrastructure is wired, this test will:
    // 1. Define a simple calculator tool
    // 2. Create a ReAct agent with Ollama
    // 3. Ask a math question
    // 4. Verify the agent calls the calculator tool
    // 5. Verify the final answer is correct
}

#[ignore = "requires running Ollama instance"]
#[tokio::test]
async fn e2e_agent_multi_turn() {
    // Stub: test multi-turn agent conversation with tool use.
}
