# Reference

This section is the information-oriented API reference for the Synwire Agent Core Runtime.
The primary reference is the rustdoc output — see [**Generating API docs**](#generating-api-docs)
below. The pages here give a structured index and prose descriptions for practitioners who
know what they are looking for.

## Contents

| File | Covers |
|------|--------|
| [traits.md](traits.md) | All public traits — agent, backend, MCP, session, store |
| [types.md](types.md) | All public structs and enums — directives, events, errors, session, backend, MCP, config |

## API Item Index

### `synwire_core::agents`

| Item | Kind | File |
|------|------|------|
| `AgentNode` | trait | [traits.md](traits.md#agentnode) |
| `ExecutionStrategy` | trait | [traits.md](traits.md#executionstrategy) |
| `GuardCondition` | trait | [traits.md](traits.md#guardcondition) |
| `StrategySnapshot` | trait | [traits.md](traits.md#strategysnapshot) |
| `DirectiveFilter` | trait | [traits.md](traits.md#directivefilter) |
| `DirectiveExecutor` | trait | [traits.md](traits.md#directiveexecutor) |
| `DirectivePayload` | trait | [traits.md](traits.md#directivepayload) |
| `Middleware` | trait | [traits.md](traits.md#middleware) |
| `Plugin` | trait | [traits.md](traits.md#plugin) |
| `PluginStateKey` | trait | [traits.md](traits.md#pluginstatekey) |
| `SignalRouter` | trait | [traits.md](traits.md#signalrouter) |
| `SessionManager` | trait | [traits.md](traits.md#sessionmanager) |
| `ModelProvider` | trait | [traits.md](traits.md#modelprovider) |
| `Agent<O>` | struct (builder) | [types.md](types.md#agento) |
| `RunContext` | struct | [types.md](types.md#runcontext) |
| `Runner<O>` | struct | [types.md](types.md#runnero) |
| `RunnerConfig` | struct | [types.md](types.md#runnerconfig) |
| `Directive` | enum | [types.md](types.md#directive) |
| `DirectiveResult<S>` | struct | [types.md](types.md#directiveresults) |
| `AgentEvent` | enum | [types.md](types.md#agentevent) |
| `AgentEventStream` | type alias | [types.md](types.md#agenteventstream) |
| `TerminationReason` | enum | [types.md](types.md#terminationreason) |
| `TaskEventKind` | enum | [types.md](types.md#taskeventkind) |
| `StopKind` | enum | [types.md](types.md#stopkind) |
| `RunErrorAction` | enum | [types.md](types.md#runerroraction) |
| `AgentError` | enum | [types.md](types.md#agenterror) |
| `ModelError` | enum | [types.md](types.md#modelerror) |
| `StrategyError` | enum | [types.md](types.md#strategyerror) |
| `DirectiveError` | enum | [types.md](types.md#directiveerror) |
| `FilterDecision` | enum | [types.md](types.md#filterdecision) |
| `FilterChain` | struct | [types.md](types.md#filterchain) |
| `ModelInfo` | struct | [types.md](types.md#modelinfo) |
| `ModelCapabilities` | struct | [types.md](types.md#modelcapabilities) |
| `EffortLevel` | enum | [types.md](types.md#effortlevel) |
| `ThinkingConfig` | enum | [types.md](types.md#thinkingconfig) |
| `Usage` | struct | [types.md](types.md#usage) |
| `OutputMode` | enum | [types.md](types.md#outputmode) |
| `SystemPromptConfig` | enum | [types.md](types.md#systempromptconfig) |
| `PermissionMode` | enum | [types.md](types.md#permissionmode) |
| `PermissionBehavior` | enum | [types.md](types.md#permissionbehavior) |
| `PermissionRule` | struct | [types.md](types.md#permissionrule) |
| `SandboxConfig` | struct | [types.md](types.md#sandboxconfig) |
| `NetworkConfig` | struct | [types.md](types.md#networkconfig) |
| `FilesystemConfig` | struct | [types.md](types.md#filesystemconfig) |
| `Signal` | struct | [types.md](types.md#signal) |
| `SignalKind` | enum | [types.md](types.md#signalkind) |
| `SignalRoute` | struct | [types.md](types.md#signalroute) |
| `Action` | enum | [types.md](types.md#action) |
| `ComposedRouter` | struct | [types.md](types.md#composedrouter) |
| `Session` | struct | [types.md](types.md#session) |
| `SessionMetadata` | struct | [types.md](types.md#sessionmetadata) |
| `HookRegistry` | struct | [types.md](types.md#hookregistry) |
| `HookMatcher` | struct | [types.md](types.md#hookmatcher) |
| `HookResult` | enum | [types.md](types.md#hookresult) |
| `PluginStateMap` | struct | [types.md](types.md#pluginstatemap) |
| `PluginHandle<P>` | struct | [types.md](types.md#pluginhandlep) |
| `PluginInput` | struct | [types.md](types.md#plugininput) |
| `MiddlewareInput` | struct | [types.md](types.md#middlewareinput) |
| `MiddlewareResult` | enum | [types.md](types.md#middlewareresult) |
| `MiddlewareStack` | struct | [types.md](types.md#middlewarestack) |
| `ClosureGuard` | struct | [types.md](types.md#closureguard) |
| `FsmStateId` | struct | [types.md](types.md#fsmstateid) |
| `ActionId` | struct | [types.md](types.md#actionid) |
| `ModelErrorAction` | enum | [types.md](types.md#modelerroraction) |

### `synwire_core::vfs`

| Item | Kind | File |
|------|------|------|
| `Vfs` | trait | [traits.md](traits.md#backendprotocol) |
| `SandboxVfs` | trait | [traits.md](traits.md#sandboxbackendprotocol) |
| `ApprovalCallback` | trait | [traits.md](traits.md#approvalcallback) |
| `VfsCapabilities` | bitflags | [types.md](types.md#backendcapabilities) |
| `DirEntry` | struct | [types.md](types.md#direntry) |
| `FileContent` | struct | [types.md](types.md#filecontent) |
| `WriteResult` | struct | [types.md](types.md#writeresult) |
| `EditResult` | struct | [types.md](types.md#editresult) |
| `GrepMatch` | struct | [types.md](types.md#grepmatch) |
| `GlobEntry` | struct | [types.md](types.md#globentry) |
| `TransferResult` | struct | [types.md](types.md#transferresult) |
| `FileInfo` | struct | [types.md](types.md#fileinfo) |
| `ExecuteResponse` | struct | [types.md](types.md#executeresponse) |
| `ProcessInfo` | struct | [types.md](types.md#processinfo) |
| `JobInfo` | struct | [types.md](types.md#jobinfo) |
| `ArchiveEntry` | struct | [types.md](types.md#archiveentry) |
| `ArchiveInfo` | struct | [types.md](types.md#archiveinfo) |
| `PipelineStage` | struct | [types.md](types.md#pipelinestage) |
| `GrepOptions` | struct | [types.md](types.md#grepoptions) |
| `GrepOutputMode` | enum | [types.md](types.md#grepoutputmode) |
| `ApprovalDecision` | enum | [types.md](types.md#approvaldecision) |
| `ApprovalRequest` | struct | [types.md](types.md#approvalrequest) |
| `RiskLevel` | enum | [types.md](types.md#risklevel) |
| `AutoApproveCallback` | struct | [types.md](types.md#autoapprove--autodeny-callbacks) |
| `AutoDenyCallback` | struct | [types.md](types.md#autoapprove--autodeny-callbacks) |
| `MemoryProvider` | struct | [types.md](types.md#statebackend) |
| `VfsError` | enum | [types.md](types.md#backenderror) |

### `synwire_core::mcp`

| Item | Kind | File |
|------|------|------|
| `McpTransport` | trait | [traits.md](traits.md#mcptransport) |
| `OnElicitation` | trait | [traits.md](traits.md#onelicitation) |
| `McpServerConfig` | enum | [types.md](types.md#mcpserverconfig) |
| `McpServerStatus` | struct | [types.md](types.md#mcpserverstatus) |
| `McpToolDescriptor` | struct | [types.md](types.md#mcptooldescriptor) |
| `McpConnectionState` | enum | [types.md](types.md#mcpconnectionstate) |
| `ElicitationRequest` | struct | [types.md](types.md#elicitationrequest) |
| `ElicitationResult` | enum | [types.md](types.md#elicitationresult) |
| `CancelAllElicitations` | struct | [types.md](types.md#cancelallelicitations) |

### `synwire_agent` — implementations

| Item | Kind | File |
|------|------|------|
| `BaseStore` | trait | [traits.md](traits.md#basestore) |
| `FsmStrategy` | struct | [types.md](types.md#fsmstrategy) |
| `FsmStrategyWithRoutes` | struct | [types.md](types.md#fsmstrategywithroutes) |
| `FsmStrategyBuilder` | struct | [types.md](types.md#fsmstrategybuilder) |
| `FsmTransition` | struct | [types.md](types.md#fsmtransition) |
| `DirectStrategy` | struct | [types.md](types.md#directstrategy) |
| `InMemorySessionManager` | struct | [types.md](types.md#inmemorysessionmanager) |
| `InMemoryStore` | struct | [types.md](types.md#inmemorystore) |
| `StoreProvider` | struct | [types.md](types.md#storebackend) |
| `CompositeProvider` | struct | [types.md](types.md#compositebackend) |
| `Mount` | struct | [types.md](types.md#mount) |
| `ThresholdGate` | struct | [types.md](types.md#thresholdgate) |
| `McpLifecycleManager` | struct | [types.md](types.md#mcplifecyclemanager) |
| `StdioMcpTransport` | struct | [types.md](types.md#stdio--http--inprocess-mcp-transports) |
| `HttpMcpTransport` | struct | [types.md](types.md#stdio--http--inprocess-mcp-transports) |
| `InProcessMcpTransport` | struct | [types.md](types.md#stdio--http--inprocess-mcp-transports) |
| `SummarisationMiddleware` | struct | [types.md](types.md#summarisationmiddleware) |
| `SummarisationThresholds` | struct | [types.md](types.md#summarisationthresholds) |

---

## Generating API docs

```
cargo doc --open -p synwire-core -p synwire-agent
```

The command above builds rustdoc for both crates and opens the result in the
default browser. Use `--no-deps` to skip dependency documentation when
iterating locally:

```
cargo doc --no-deps --open -p synwire-core -p synwire-agent
```

All public items carry rustdoc comments; the generated output is the definitive
source of truth for signatures, trait bounds, and detailed implementation notes.
