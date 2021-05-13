use criterion::{black_box, criterion_group, criterion_main, Criterion};
use streamson_lib::{
    matcher,
    strategy::{self, Strategy},
};

const ITEM_COUNT: usize = 10000;
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
    c.benchmark_group("Extract")
}

fn run_group(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    mut extract: strategy::Extract,
) {
    let input = gen_input(INPUT_BUFFER_SIZE);

    group.bench_function(name, |b| {
        b.iter(|| {
            for data in &input {
                extract.process(black_box(data)).unwrap();
            }
        })
    });
}

pub fn combinator(c: &mut Criterion) {
    let mut extract = strategy::Extract::new();

    let first_matcher = matcher::Combinator::new(matcher::Depth::new(1, Some(1)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"logs"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;

    extract.add_matcher(Box::new(first_combo.clone()), None);
    extract.add_matcher(Box::new(second_combo.clone()), None);

    let mut group = get_benchmark_group(c);
    run_group(&mut group, "Combinator", extract);

    let mut extract = strategy::Extract::new();
    let first_matcher = matcher::Combinator::new(matcher::Depth::new(40, Some(60)));
    let second_matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"none"}[]"#).unwrap());
    let first_combo = first_matcher.clone() | second_matcher.clone();
    let second_combo = first_matcher & !second_matcher;
    extract.add_matcher(Box::new(first_combo), None);
    extract.add_matcher(Box::new(second_combo), None);
    run_group(&mut group, "Combinator-NoMatch", extract);

    group.finish();
}

pub fn void(c: &mut Criterion) {
    let mut group = get_benchmark_group(c);

    let extract = strategy::Extract::new();
    run_group(&mut group, "Void", extract);
}
criterion_group!(benches, void, combinator);
criterion_main!(benches);
