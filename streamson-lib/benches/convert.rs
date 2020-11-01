use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use streamson_lib::{handler, matcher, strategy};

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
    c.benchmark_group("Filter")
}

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut convert: strategy::Convert,
) {
    let input = gen_input(INPUT_BUFFER_SIZE);

    group.bench_function(name, |b| {
        b.iter(|| {
            for data in &input {
                convert.process(black_box(data)).unwrap();
            }
        })
    });
}

pub fn combinator(c: &mut Criterion) {
    let mut convert = strategy::Convert::new();
    let replace_handler = Arc::new(Mutex::new(handler::Replace::new(
        r#""***""#.as_bytes().iter().copied().collect(),
    )));

    let first_matcher = matcher::Combinator::new(matcher::Depth::new(1, Some(1)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"logs"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;

    convert.add_matcher(Box::new(first_combo.clone()), vec![replace_handler.clone()]);
    convert.add_matcher(
        Box::new(second_combo.clone()),
        vec![replace_handler.clone()],
    );

    let mut group = get_benchmark_group(c);
    run_group(&mut group, "Combinator", convert);

    let mut convert = strategy::Convert::new();
    let first_matcher = matcher::Combinator::new(matcher::Depth::new(40, Some(60)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"none"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    convert.add_matcher(Box::new(first_combo), vec![replace_handler.clone()]);
    convert.add_matcher(Box::new(second_combo), vec![replace_handler.clone()]);
    run_group(&mut group, "Combinator-NoMatch", convert);

    group.finish();
}

pub fn void(c: &mut Criterion) {
    let mut group = get_benchmark_group(c);

    let convert = strategy::Convert::new();
    run_group(&mut group, "Void", convert);
}
criterion_group!(benches, void, combinator);
criterion_main!(benches);
