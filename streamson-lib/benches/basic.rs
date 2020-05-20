use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{handler, matcher, Collector};

pub fn simple(c: &mut Criterion) {
    let mut collector = Collector::new();

    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#);
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#);
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));

    collector = collector.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    collector = collector.add_matcher(Box::new(second_matcher), &[handler.clone()]);

    let mut input = Vec::new();
    input.push(br#"{ "users": ["#.to_vec());
    for _ in 0..5000 {
        input.push(br#"{"a": "c"}"#.to_vec());
    }
    input.push(br#""last"], {"logs":"#.to_vec());
    for _ in 0..1000 {
        input.push(br#"{"l": "ll"}"#.to_vec());
    }
    input.push(br#""last"]}"""#.to_vec());
    let input = vec![b"{".to_vec(), b"}".to_vec()];

    let mut group = c.benchmark_group("simple");
    group.bench_function("simple buffer handler", |b| {
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

criterion_group!(benches, simple);
criterion_main!(benches);
