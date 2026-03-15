//! Benchmark: Measure overhead of core type construction and serialization.
//!
//! When observability (tracing) is wired, this benchmark will measure
//! span creation overhead. For now it benchmarks message construction
//! and serde roundtrip as a baseline.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Benchmark harness provides its own main.
#![allow(missing_docs, unused_results)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use synwire_core::messages::Message;

fn bench_message_construction(c: &mut Criterion) {
    c.bench_function("Message::human construction", |b| {
        b.iter(|| {
            let msg = Message::human(black_box("Hello, world!"));
            black_box(msg);
        });
    });

    c.bench_function("Message::ai construction", |b| {
        b.iter(|| {
            let msg = Message::ai(black_box("Response text"));
            black_box(msg);
        });
    });
}

fn bench_message_serde_roundtrip(c: &mut Criterion) {
    let msg = Message::human("Hello, this is a benchmark test message with some content.");

    c.bench_function("Message serde roundtrip", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&msg)).unwrap();
            let _roundtripped: Message = serde_json::from_str(black_box(&json)).unwrap();
        });
    });

    let ai_msg = Message::ai("AI response with tool calls and metadata for benchmarking.");

    c.bench_function("AI Message serde roundtrip", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&ai_msg)).unwrap();
            let _roundtripped: Message = serde_json::from_str(black_box(&json)).unwrap();
        });
    });
}

fn bench_message_content_extraction(c: &mut Criterion) {
    let msg = Message::human("Extract this content for benchmarking purposes.");

    c.bench_function("Message content().as_text()", |b| {
        b.iter(|| {
            let text = black_box(&msg).content().as_text();
            black_box(text);
        });
    });
}

fn span_overhead_baseline(c: &mut Criterion) {
    c.bench_function("noop_baseline", |b| {
        b.iter(|| {
            std::hint::black_box(42);
        });
    });
}

criterion_group!(
    benches,
    span_overhead_baseline,
    bench_message_construction,
    bench_message_serde_roundtrip,
    bench_message_content_extraction,
);
criterion_main!(benches);
