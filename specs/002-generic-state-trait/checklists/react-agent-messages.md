# Checklist: MessagesState + ReAct Agent + ToolNode

**Purpose**: Verify MessagesState works with create_react_agent and ToolNode out of the box
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S08, FR-S09, FR-S10, SC-S03

## MessagesState (FR-S08)

- [ ] `MessagesState` struct has `messages: Vec<Message>` field
- [ ] `messages` field uses Topic channel (append semantics)
- [ ] `MessagesState` derives or implements `State`
- [ ] `MessagesState` derives `Clone`, `Serialize`, `Deserialize`
- [ ] Two nodes each appending a message results in both messages in order

## create_react_agent (FR-S09)

- [ ] Returns `CompiledGraph<MessagesState>` (not `CompiledGraph` or `CompiledGraph<ValueState>`)
- [ ] Agent node receives `MessagesState`, invokes model with `state.messages`, appends AI response
- [ ] Tools condition inspects `state.messages` for tool calls on last AI message
- [ ] Compiles with `FakeChatModel` and empty tools list
- [ ] Invoke with human message returns `MessagesState` with human + AI messages

## ToolNode (FR-S10)

- [ ] `into_node_fn<S: State>()` is generic (not hardcoded to MessagesState or Value)
- [ ] Extracts messages from `S` via serde
- [ ] Executes tool calls and constructs tool-response messages
- [ ] Injects tool messages back into `S` via serde
- [ ] Truncation (`max_result_size`) still works with generic state
- [ ] Unknown tool returns `GraphError::ToolNotFound`

## Integration (SC-S03)

- [ ] Full ReAct loop: human message → agent (no tools) → END produces correct MessagesState
- [ ] Full ReAct loop: human message → agent (with tools) → tools → agent → END produces correct MessagesState
- [ ] Tool results appear as ToolMessage in `state.messages`

## Notes

- Tasks: T024-T035
