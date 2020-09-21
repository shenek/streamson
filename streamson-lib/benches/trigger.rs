use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{handler, matcher, strategy};

const INPUT_BUFFER_SIZE: usize = 1024;
const ITEM_COUNT: usize = 10_000;

fn gen_input(size: usize) -> Vec<Vec<u8>> {
    let mut all_in_one = vec![];

    all_in_one.extend(br#"{ "users": ["#.to_vec());
    for _ in 0..ITEM_COUNT / 2 - 1 {
        all_in_one.extend(br#"{"a": "c"},"#.to_vec());
    }
    all_in_one.extend(br#""last"], "logs": ["#.to_vec());
    for _ in 0..ITEM_COUNT / 2 - 1 {
        all_in_one.extend(br#"{"l": "ll"},"#.to_vec());
    }
    all_in_one.extend(br#""last"]}"""#.to_vec());
    all_in_one.extend(br#"}"#.to_vec());

    all_in_one.chunks(size).map(|e| e.to_vec()).collect()
}

fn get_benchmark_group(
    c: &mut Criterion,
) -> criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> {
    c.benchmark_group("Trigger")
}

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut trigger: strategy::Trigger,
    handler: Arc<Mutex<handler::Buffer>>,
    expected_count: usize,
) {
    let input = gen_input(INPUT_BUFFER_SIZE);
    let mut count = 0;
    group.bench_function(name, |b| {
        b.iter(|| {
            for data in &input {
                trigger.process(black_box(data)).unwrap();
                let mut guard = handler.lock().unwrap();
                while let Some((_path, _data)) = guard.pop() {
                    count += 1;
                }
            }
        })
    });
    if count != expected_count {
        panic!("Count {}!={}", count, expected_count)
    }
}

pub fn simple(c: &mut Criterion) {
    let mut trigger = strategy::Trigger::new();

    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(false)));

    trigger.add_matcher(Box::new(first_matcher.clone()), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher.clone()), &[handler.clone()]);

    let mut group = get_benchmark_group(c);
    run_group(
        &mut group,
        "Simple-Buffer(no path)",
        trigger,
        handler,
        ITEM_COUNT,
    );

    let mut trigger = strategy::Trigger::new();
    let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"logs"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));

    trigger.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(
        &mut group,
        "Simple-Buffer(with path)",
        trigger,
        handler,
        ITEM_COUNT,
    );

    let mut trigger = strategy::Trigger::new();
    let first_matcher = matcher::Simple::new(r#"{"not-found"}[]"#).unwrap();
    let second_matcher = matcher::Simple::new(r#"{"found-not"}[]"#).unwrap();
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));
    trigger.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(&mut group, "Simple-NoMatch", trigger, handler, 0);

    group.finish();
}

pub fn depth(c: &mut Criterion) {
    let mut trigger = strategy::Trigger::new();

    let first_matcher = matcher::Depth::new(1, None);
    let second_matcher = matcher::Depth::new(1, Some(1));
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(false)));

    trigger.add_matcher(Box::new(first_matcher.clone()), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher.clone()), &[handler.clone()]);

    let mut group = get_benchmark_group(c);
    run_group(
        &mut group,
        "Depth-Buffer(no path)",
        trigger,
        handler,
        ITEM_COUNT * 2 + 2,
    );

    let mut trigger = strategy::Trigger::new();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
    trigger.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(
        &mut group,
        "Depth-Buffer(with path)",
        trigger,
        handler,
        ITEM_COUNT * 2 + 2,
    );

    let mut trigger = strategy::Trigger::new();
    let first_matcher = matcher::Depth::new(50, None);
    let second_matcher = matcher::Depth::new(40, Some(60));
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));
    trigger.add_matcher(Box::new(first_matcher), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_matcher), &[handler.clone()]);
    run_group(&mut group, "Depth-NoMatch", trigger, handler, 0);

    group.finish();
}

pub fn combinator(c: &mut Criterion) {
    let mut trigger = strategy::Trigger::new();

    let first_matcher = matcher::Combinator::new(matcher::Depth::new(1, Some(1)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"logs"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(false)));

    trigger.add_matcher(Box::new(first_combo.clone()), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_combo.clone()), &[handler.clone()]);

    let mut group = get_benchmark_group(c);
    run_group(
        &mut group,
        "Combinator-Buffer(no path)",
        trigger,
        handler,
        ITEM_COUNT / 2 + 4,
    );

    let mut trigger = strategy::Trigger::new();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
    trigger.add_matcher(Box::new(first_combo), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_combo), &[handler.clone()]);
    run_group(
        &mut group,
        "Combinator-Buffer(with path)",
        trigger,
        handler,
        ITEM_COUNT / 2 + 4,
    );

    let mut trigger = strategy::Trigger::new();
    let first_matcher = matcher::Combinator::new(matcher::Depth::new(40, Some(60)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"none"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    let handler = Arc::new(Mutex::new(handler::Buffer::new()));
    trigger.add_matcher(Box::new(first_combo), &[handler.clone()]);
    trigger.add_matcher(Box::new(second_combo), &[handler.clone()]);
    run_group(&mut group, "Combinator-NoMatch", trigger, handler, 0);

    group.finish();
}

pub fn void(c: &mut Criterion) {
    let trigger = strategy::Trigger::new();
    let mut group = get_benchmark_group(c);
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(false)));
    run_group(&mut group, "Void-Buffer(no path)", trigger, handler, 0);

    let trigger = strategy::Trigger::new();
    let handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
    run_group(&mut group, "Void-Buffer(with path)", trigger, handler, 0);
}

criterion_group!(benches, simple, depth, combinator, void);
criterion_main!(benches);
