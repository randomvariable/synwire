# Provider Integrations

## Overview

Synwire's core abstractions — `VectorStore`, `DocumentLoader`, `Retriever`, `Embeddings`, `ChatModel` — are trait-based, allowing provider implementations to be swapped without altering application logic. This document covers the planned concrete provider implementations: the search backends, vector stores, databases, and orchestration engines that Synwire will ship as first-class integrations.

Each provider lives in its own crate with a single dependency on `synwire-core` traits, keeping the dependency graph narrow and compile times manageable.

## Search Providers

### Serper

Serper provides Google Search API access, giving agents a structured way to query the web. The `synwire-serper` crate implements a search tool suitable for direct use inside agent tool registries.

Key characteristics:

- API key authentication (via environment variable or explicit config)
- Configurable result count and search type (search, images, news)
- Structured snippet extraction — results are returned as typed structs, not raw HTML
- Rate limiting support to stay within API quotas

### SearXNG

SearXNG is a self-hosted metasearch engine that aggregates results from multiple search engines without tracking. The `synwire-searxng` crate targets deployments where privacy or data sovereignty is a concern.

Key characteristics:

- No API key requirement — authentication is handled at the instance level
- Configurable instance URL, allowing teams to point at their own deployment
- Category and engine selection (general, science, files, etc.)
- Language and region filtering for localised results

## Vector Store Providers

### Qdrant

Qdrant is a high-performance vector similarity search engine purpose-built for embedding-based retrieval. The `synwire-qdrant` crate implements the `VectorStore` trait with full CRUD operations.

Key capabilities:

- Similarity search with configurable distance metrics (cosine, dot, Euclidean)
- Metadata filtering via the `MetadataFilter` algebra (FR-339): `Eq`, `Ne`, `In`, `Gt`, `Lt`, `And`, `Or` predicates map directly to Qdrant's filtering DSL
- Collection management — create, delete, and configure collections programmatically
- Payload indexing for efficient filtered searches
- Both gRPC and REST transport options, selectable at construction time

### pgvector (PostgreSQL)

pgvector brings vector similarity search to PostgreSQL, making it attractive for teams that already run Postgres and want to avoid adding a dedicated vector database. The `synwire-pgvector` crate implements the `VectorStore` trait.

Key capabilities:

- IVFFlat and HNSW indexing strategies, configurable per collection
- Metadata filtering through the same `MetadataFilter` interface (FR-339), translated to SQL `WHERE` clauses
- Connection pooling via `sqlx` for production-grade concurrency
- Leverages existing PostgreSQL infrastructure — backups, replication, and access control come for free

### Neo4j

Neo4j combines graph database capabilities with vector search, enabling knowledge-graph-aware RAG patterns. The `synwire-neo4j` crate implements the `VectorStore` trait for vector index queries while also exposing graph traversal for hybrid retrieval.

Key capabilities:

- Vector index queries through Neo4j's native vector search
- Graph-aware retrieval — follow relationships from vector search results to discover contextually related nodes
- Cypher query integration for hybrid graph+vector retrieval pipelines
- Bolt protocol communication via the `neo4rs` driver
- Suitable for knowledge graph RAG where entity relationships matter as much as semantic similarity

## Database Providers

### PostgreSQL

Beyond pgvector's embedding-specific use case, PostgreSQL serves as the general-purpose relational backend for several Synwire persistence concerns:

- **Checkpoint persistence** — implements `CheckpointSaver` with format versioning (FR-323), migration support (FR-324), and configurable maximum checkpoint size (FR-348)
- **Session storage** — implements `SessionProvider` for create/save/load/delete session lifecycle management (FR-387)
- **Store backend** — implements `BaseStore` for key-value persistence with scoped prefixes (`app:`, `user:`, `session:`, `temp:`) as defined in FR-150
- **Multi-tenant isolation** — all database-backed storage supports `tenant_id` scoping (FR-327, FR-328) to ensure data separation across tenants

Infrastructure concerns:

- Connection pooling via `sqlx` with configurable pool size and idle timeout
- Schema migrations managed through embedded SQL files
- Transaction support for atomic checkpoint + session updates

## Workflow Orchestration

### Temporal

Temporal provides durable workflow execution for long-running agent tasks that must survive process restarts, network failures, or deployment rollouts. The `synwire-temporal` crate bridges Synwire's agent model with Temporal's workflow primitives.

Key capabilities:

- **Workflow-as-agent pattern** — Temporal workflows implement the `AgentNode` trait, making orchestrated workflows composable with standard Synwire graphs
- **Activity-based tool execution** — each tool invocation runs as a Temporal activity with automatic retries, timeouts, and heartbeating
- **Signal-based HITL flows** — human-in-the-loop approval steps use Temporal signals, allowing workflows to pause and resume on human input
- **Durable state** — workflow state survives crashes without requiring explicit checkpointing
- Integration via `temporal-sdk-core`, the official Rust SDK for the Temporal platform

## Core Trait Requirements

All provider implementations target abstract traits defined in `synwire-core`. These traits establish the contracts that providers must satisfy:

| Trait | Purpose | Key FRs |
|-------|---------|---------|
| `VectorStore` | CRUD + similarity search with metadata filtering | FR-339 |
| `DocumentLoader` | Document extraction (`TextLoader`, `JsonLoader`, `CsvLoader`) | FR-336 |
| `Retriever` | Retrieval modes: dense, sparse, hybrid | FR-338 |
| `Reranker` | Post-retrieval document reordering | FR-337 |
| `KnowledgeBase` | Unified RAG: document sources + vector store + query method | FR-373 |
| `CheckpointSaver` | Checkpoint format versioning, migration, max size enforcement | FR-323, FR-324, FR-348 |
| `SessionProvider` | Create/save/load/delete session lifecycle | FR-387 |
| `BaseStore` | Key-value persistence with scoped prefixes | FR-150 |

Providers implement only the traits relevant to their capabilities — a vector store crate does not need to implement `CheckpointSaver`, and a database crate does not need to implement `Retriever`.

## Success Criteria

- **SC-053**: `MetadataFilter` on `VectorStore` correctly filters results across all vector store providers
- Each provider passes a conformance test suite for its implemented traits, run as integration tests against real service instances
- Connection pooling and timeout handling verified under load for all network-backed providers
- Multi-tenant isolation verified for database-backed providers — queries scoped to one `tenant_id` must never return data from another

## Crate Organisation

| Crate | Providers | Core Trait Dependencies |
|-------|-----------|------------------------|
| `synwire-serper` | Serper search | Search tool interface |
| `synwire-searxng` | SearXNG search | Search tool interface |
| `synwire-qdrant` | Qdrant vector store | `VectorStore` |
| `synwire-pgvector` | pgvector vector store | `VectorStore` |
| `synwire-neo4j` | Neo4j vector + graph store | `VectorStore` |
| `synwire-postgres` | PostgreSQL checkpoint/session/store | `CheckpointSaver`, `SessionProvider`, `BaseStore` |
| `synwire-temporal` | Temporal workflow orchestration | `AgentNode` |

All provider crates depend only on `synwire-core` traits — not the full SDK. This keeps provider dependencies isolated and allows consumers to pull in only the backends they need.
