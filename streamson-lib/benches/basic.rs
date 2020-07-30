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

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut collector: Collector,
    handler: Arc<Mutex<handler::Buffer>>,
) {
    let input = gen_input();
    group.bench_function(name, |b| {
        b.iter(|| {
            for data in &input {
                collector.process(black_box(data)).unwrap();
                let mut guard = handler.lock().unwrap();
                while let Some((_, _)) = guard.pop() {}
            }
        })
    });
}

pub fn simple(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(false)));

    collector = collector.add_matcher(Box::new(first_matcher.clone()), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher.clone()), &[handler.clone()]);

    let mut group = c.benchmark_group("Simple");
    run_group(&mut group, "Buffer(no path)", collector, handler);

    let mut collector = Collector::new();
    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(true)));

    collector = collector.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(&mut group, "Buffer(with path)", collector, handler);

    group.finish();
}

pub fn depth(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Depth::new(1, None);
    let second_matcher = matcher::Depth::new(1, Some(1));
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(false)));

    collector = collector.add_matcher(Box::new(first_matcher.clone()), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher.clone()), &[handler.clone()]);

    let mut group = c.benchmark_group("Depth");
    run_group(&mut group, "Buffer(no path)", collector, handler);

    let mut collector = Collector::new();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(true)));
    collector = collector.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(&mut group, "Buffer(with path)", collector, handler);

    group.finish();
}

pub fn combinator(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Combinator::new(matcher::Depth::new(1, None));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"logs"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(false)));

    collector = collector.add_matcher(Box::new(first_combo.clone()), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_combo.clone()), &[handler.clone()]);

    let mut group = c.benchmark_group("Combinator");
    run_group(&mut group, "Buffer(no path)", collector, handler);

    let mut collector = Collector::new();
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));
    collector = collector.add_matcher(Box::new(first_combo), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_combo), &[handler.clone()]);
    run_group(&mut group, "Buffer(with path)", collector, handler);

    group.finish();
}

criterion_group!(benches, simple, depth, combinator);
criterion_main!(benches);
