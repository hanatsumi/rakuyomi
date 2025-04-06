use shared::{settings::Settings, source::Source};
#[allow(unused_imports)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::{env, path::PathBuf};
use tokio_util::sync::CancellationToken;

pub fn search_mangas_benchmark(c: &mut Criterion) {
    let source_path: PathBuf = env::var("BENCHMARK_SOURCE_PATH").unwrap().into();
    let query = env::var("BENCHMARK_QUERY").unwrap();
    let settings = Settings::default();

    let source = Source::from_aix_file(source_path.as_ref(), settings).unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("search_mangas", |b| {
        b.to_async(&runtime)
            .iter(|| source.search_mangas(CancellationToken::new(), query.clone()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = search_mangas_benchmark
}
criterion_main!(benches);
