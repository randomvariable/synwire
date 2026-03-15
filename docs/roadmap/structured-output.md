# Structured Output & DSPy

## Overview
Covers two related areas: the structured output lifecycle (typed extraction from LLM responses) and DSPy-style prompt programming (learnable prompt optimisation). Content from instructor-mirascope-parity and dspy-prompt-handling research.

## Structured Output Lifecycle (FR-436-464)

### Validation & Retry (FR-436, FR-440-441)
- Reask with validation feedback — retry loop includes validation error in next LLM prompt. Configurable error formatter (FR-436)
- `Maybe<T>` wrapper — always succeeds deserialisation, populates error when extraction fails (FR-440)
- `LlmValidator` trait for semantic validation via LLM call. Composable with guardrails (FR-441)

### Streaming Extraction (FR-437-438)
- `PartialStream<T>` yielding progressively-filled instances as streaming tokens arrive (FR-437)
- `IterableStream<T>` extracting a sequence of typed objects from a single streaming response (FR-438)

### Output Modes (FR-135, FR-443, FR-464)
- `OutputMode<T>` enum: Tool, Native, Prompt, Custom. Tool is default/universal (FR-135)
- Validates mode/provider compatibility at construction time via ModelProfile (FR-443)
- Fallback chain: Native → Tool → Prompt. Configurable (FR-383)
- Distinguishes JSON parse failures from schema validation failures (FR-464)

### Convenience Parsers (FR-442)
- XML, Regex, CSV, Enum, Combining output parsers in synwire convenience crate

### Developer Ergonomics (FR-444-450)
- `#[call]` attribute macro generating LLM invocation function from annotated async function (FR-444)
- `#[prompt]` attribute macro generating type-safe PromptTemplate from function signature (FR-445)
- `Agent::resume()` for ergonomic in-process multi-turn continuation (FR-446)
- `ModelProfile` type with capability fields: native JSON, tool calling, streaming, vision, audio, max tokens, caching (FR-447)
- Runtime model override via with_model() and RunConfig model_override (FR-448)
- `PromptVersion` metadata type for prompt template versioning (FR-449)
- `with_retry` supports optional retry_model for "escalate to stronger model on failure" patterns (FR-450)

### Provider Abstraction (FR-458-459)
- `ModelProfileRegistry` trait with register/get/supports methods (FR-458)
- Runtime provider registration (FR-459)

### Operational Patterns (FR-451-455)
- `BatchProcessor<T>` for provider batch APIs (FR-451)
- `BaseCache` usable for caching structured extraction results (FR-452)
- `ExtractionCollector` for recording input/output pairs for fine-tuning (FR-453)
- `GraphExecutionMetrics` includes token usage from all attempts including retries (FR-454)
- `Vec<FailedAttempt>` history during execution (FR-455)

### Additional (FR-460-463)
- Multimodal input support in run::<T>() (FR-460)
- Reasoning/thinking content preserved alongside structured output (FR-461)
- ExtractionMetadata on successful results (FR-462)
- Deeply nested type support with correct JSON schema generation (FR-463)

## DSPy-Style Prompt Programming (FR-404-435)

### Signatures (FR-404-409)
- `Signature` trait with instruction, InputField entries, OutputField entries. Complementary to PromptTemplate (FR-404)
- `InputField<T>` and `OutputField<T>` with desc, prefix, format metadata (FR-405)
- `#[derive(Signature)]` proc macro (FR-406)
- `Signature::from_str` shorthand: "question -> answer" (FR-407)
- Default instruction generation from field metadata (FR-408)
- Signature composition: prepend() and append() for field injection (FR-409)

### Predict Modules (FR-410-417)
- `Module` trait with forward()/aforward(), learnable parameters, dump_state()/load_state(). All Modules implement Runnable (FR-410)
- `Prediction` type as named-field output container (FR-411)
- `Predict` module: format inputs → invoke LM → parse outputs via Adapter (FR-412)
- `ChainOfThought` prepending reasoning OutputField for step-by-step reasoning (FR-413)
- `BestOfN` generating N completions and selecting highest-scoring by metric (FR-414)
- `Refine` iterating predictions with evaluation feedback up to max_iterations (FR-415)
- `ProgramOfThought` prompting LM to generate executable code (FR-416)
- `Parallel` predict running concurrent predict calls via tokio::JoinSet (FR-417)

### Adapters (FR-418-423)
- `Adapter` trait with format_prompt() and parse_response() for Signature ↔ LM message translation (FR-418)
- `ChatAdapter` using delimiter patterns (e.g. [[ ## field ## ]]) (FR-419)
- `JsonAdapter` formatting as JSON schema and parsing JSON responses (FR-420)
- Automatic adapter fallback chain: Chat → JSON on parse failure (FR-421)
- Native function calling support via use_native_function_calling flag (FR-422)

### Teleprompt Optimisers (FR-424-431)
- `Teleprompter` trait with compile(module, trainset, metric) (FR-424)
- `Example` type as training/evaluation datum (FR-425)
- `Evaluate` type for scoring a module against a dataset (FR-426)
- `BootstrapFewShot` optimiser with configurable max demos (FR-427)
- `MIPRO` optimiser for joint instruction + demonstration optimisation (FR-428)
- `COPRO` signature optimiser for instruction string optimisation (FR-429)
- Module state serialisation: JSON with version field (FR-430)
- `BootstrapFinetune` compiling predictions into JSONL fine-tuning datasets (FR-431)

### Integration (FR-432-435)
- All Module types implement Runnable for chains and graphs (FR-432)
- Agent<D, O> accepts optional signature as alternative to system_prompt (FR-433)
- PromptCachingMiddleware handles signature-based prompts (FR-434)
- Documentation of few-shot templates vs DSPy demonstrations as complementary approaches (FR-435)

## Hook Deepening (FR-456-457)
- Hook composition semantics: client → agent → per-call merge order (FR-456)
- CallbackHandler structured-output lifecycle events: on_parse_start, on_parse_error, on_parse_success (FR-457)

## Guardrails (FR-353-356, FR-439)
- `InputGuardrail` trait with tripwire semantics — halts agent before model invocation. Composable, parallel (FR-353)
- `OutputGuardrail` trait validating agent's final response (FR-354)
- Per-tool guardrails via optional input_guardrails / output_guardrails on Tool config (FR-355)
- `FactualConsistencyGuardrail` documentation example (FR-356)
- Guardrails accept optional validation_context for domain-specific context (FR-439)

## Success Criteria
- **SC-069**: Signature with Predict and mock model produces typed Prediction
- **SC-070**: ChainOfThought produces reasoning + answer fields
- **SC-071**: BootstrapFewShot improves metric score on validation set
- **SC-072**: Optimised module round-trips through dump_state/load_state
- **SC-073**: Predict module works in RunnableSequence
- **SC-074**: ChatAdapter parses delimited response; fallback to JsonAdapter succeeds
- **SC-076**: synwire-dspy tests pass with ≥ 80% line coverage
- **SC-077**: Zero unsafe blocks in synwire-dspy
- **SC-078**: Reask with validation feedback corrects invalid first response
- **SC-079**: PartialStream<T> yields progressively-filled partial objects
- **SC-080**: IterableStream<T> extracts multiple typed objects from single stream
- **SC-081**: Maybe<T> returns error instead of failing on impossible extraction
- **SC-082**: LlmValidator rejects output and reask loop corrects it
- **SC-083**: ModelProfile reports capabilities; OutputMode rejects incompatible mode
- **SC-084**: #[call] macro generates working LLM invocation
- **SC-085**: Agent::resume() continues without reconstructing
- **SC-055**: Input guardrails halt execution when tripwire triggered
- **SC-056**: Output guardrails reject invalid output and agent re-tries
