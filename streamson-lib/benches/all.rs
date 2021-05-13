use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{
    handler,
    strategy::{self, Strategy},
};

const ITEM_COUNT: usize = 100;
const INPUT_BUFFER_SIZE: usize = 1024;

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
    c.benchmark_group("All")
}

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut all: strategy::All,
) {
    let input = gen_input(INPUT_BUFFER_SIZE);

    group.bench_function(name, |b| {
        b.iter(|| {
            for data in &input {
                all.process(black_box(data)).unwrap();
            }
        })
    });
}

pub fn indenter(c: &mut Criterion) {
    let mut all = strategy::All::new();
    let indent_handler = Arc::new(Mutex::new(handler::Indenter::new(Some(2))));
    all.add_handler(indent_handler.clone());

    let mut group = get_benchmark_group(c);
    run_group(&mut group, "Indenter(2)", all);

    let mut all = strategy::All::new();
    let noindent_handler = Arc::new(Mutex::new(handler::Indenter::new(None)));
    all.add_handler(noindent_handler.clone());
    run_group(&mut group, "Indenter(None)", all);

    group.finish();
}

pub fn analyser(c: &mut Criterion) {
    let mut all = strategy::All::new();
    let analyser_handler = Arc::new(Mutex::new(handler::Analyser::new()));
    all.add_handler(analyser_handler.clone());

    let mut group = get_benchmark_group(c);
    run_group(&mut group, "Analyser", all);

    group.finish();
}

pub fn void(c: &mut Criterion) {
    let mut group = get_benchmark_group(c);

    let all = strategy::All::new();
    run_group(&mut group, "Void", all);
}
criterion_group!(benches, void, indenter, analyser);
criterion_main!(benches);
