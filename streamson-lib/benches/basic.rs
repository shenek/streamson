use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{handler, matcher, Collector};

fn gen_input() -> Vec<Vec<u8>> {
    let mut input = Vec::new();
    input.push(br#"{ "users": ["#.to_vec());
    for _ in 0..5_000 {
        input.push(br#"{"a": "c"},"#.to_vec());
    }
    input.push(br#""last"], "logs": ["#.to_vec());
    for _ in 0..5_000 {
        input.push(br#"{"l": "ll"},"#.to_vec());
    }
    input.push(br#""last"]}"""#.to_vec());
    input.push(br#"}"#.to_vec());

    input
}

pub fn simple(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));

    collector = collector.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher), &[handler.clone()]);

    let input = gen_input();
    let mut group = c.benchmark_group("Simple");
    group.bench_function("Buffer", |b| {
        b.iter(|| {
            for data in &input {
                collector.process(black_box(data)).unwrap();
                let mut guard = handler.lock().unwrap();
                while let Some((_, _)) = guard.pop() {}
            }
        })
    });
    group.finish();
}

pub fn depth(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Depth::new(1, None);
    let second_matcher = matcher::Depth::new(1, Some(1));
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));

    collector = collector.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher), &[handler.clone()]);

    let input = gen_input();
    let mut group = c.benchmark_group("Depth");
    group.bench_function("Buffer", |b| {
        b.iter(|| {
            for data in &input {
                collector.process(black_box(data)).unwrap();
                let mut guard = handler.lock().unwrap();
                while let Some((_, _)) = guard.pop() {}
            }
        })
    });
    group.finish();
}

pub fn combinator(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Combinator::new(matcher::Depth::new(1, None));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"logs"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));

    collector = collector.add_matcher(Box::new(first_combo), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_combo), &[handler.clone()]);

    let input = gen_input();
    let mut group = c.benchmark_group("Combinator");
    group.bench_function("Buffer", |b| {
        b.iter(|| {
            for data in &input {
                collector.process(black_box(data)).unwrap();
                let mut guard = handler.lock().unwrap();
                while let Some((_, _)) = guard.pop() {}
            }
        })
    });
    group.finish();
}

criterion_group!(benches, simple, depth, combinator);
criterion_main!(benches);
