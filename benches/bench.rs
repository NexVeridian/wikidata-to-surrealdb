use anyhow::{Error, Ok, Result};
use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::{env, time::Duration};
use surrealdb::{engine::local::Db, Surreal};
use tokio::runtime::Runtime;

use wikidata_to_surrealdb::utils::*;

async fn inti_db() -> Result<Surreal<Db>, Error> {
    env::set_var("WIKIDATA_LANG", "en");
    env::set_var("OVERWRITE_DB", "true");

    let db = init_db::create_db_mem().await?;

    Ok(db)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Create DB Entities");

    group.bench_function("Single Insert", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let db = inti_db().await.unwrap();
                let reader = File_Format::new("json")
                    .reader("tests/data/bench.json")
                    .unwrap();

                CreateVersion::Single
                    .run_threaded(Some(db.clone()), reader, None, 1000, 100)
                    .await
                    .unwrap();
            })
        })
    });

    group.bench_function("Bulk Insert", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let db = inti_db().await.unwrap();
                let reader = File_Format::new("json")
                    .reader("tests/data/bench.json")
                    .unwrap();

                CreateVersion::Bulk
                    .run_threaded(Some(db.clone()), reader, None, 1000, 100)
                    .await
                    .unwrap();
            })
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(120, Output::Protobuf)).measurement_time(Duration::from_secs(50));
    targets= bench
}
criterion_main!(benches);
