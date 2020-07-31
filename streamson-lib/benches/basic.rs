use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{handler, matcher, Collector};

const INPUT_BUFFER_SIZE: usize = 1024;

fn gen_input(size: usize) -> Vec<Vec<u8>> {
    let mut all_in_one = vec![];

    all_in_one.extend(br#"{ "users": ["#.to_vec());
    for _ in 0..5_000 {
        all_in_one.extend(br#"{"a": "c"},"#.to_vec());
    }
    all_in_one.extend(br#""last"], "logs": ["#.to_vec());
    for _ in 0..5_000 {
        all_in_one.extend(br#"{"l": "ll"},"#.to_vec());
    }
    all_in_one.extend(br#""last"]}"""#.to_vec());
    all_in_one.extend(br#"}"#.to_vec());

    all_in_one.chunks(size).map(|e| e.to_vec()).collect()
}

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut collector: Collector,
    handler: Arc<Mutex<handler::Buffer>>,
) {
    let input = gen_input(INPUT_BUFFER_SIZE);
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
