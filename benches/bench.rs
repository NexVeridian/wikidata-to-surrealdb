use anyhow::{Error, Ok, Result};
use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::{env, time::Duration};
use surrealdb::{
    engine::local::{Db, Mem},
    Surreal,
};
use tokio::runtime::Runtime;

use wikidata_to_surrealdb::utils::*;

async fn inti_db() -> Result<Surreal<Db>, Error> {
    env::set_var("WIKIDATA_LANG", "en");
    env::set_var("OVERWRITE_DB", "true");

    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

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

                create_db_entities_threaded(
                    Some(db.clone()),
                    reader,
                    None,
                    1000,
                    100,
                    CreateVersion::Single,
                )
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

                create_db_entities_threaded(
                    Some(db.clone()),
                    reader,
                    None,
                    1000,
                    100,
                    CreateVersion::Bulk,
                )
                .await
                .unwrap();
            })
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf)).measurement_time(Duration::from_secs(60));
    targets= bench
}
criterion_main!(benches);
