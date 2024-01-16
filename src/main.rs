use anyhow::{Error, Ok, Result};
use lazy_static::lazy_static;
use std::{env, io::BufRead};
use surrealdb::{engine::remote::ws::Client, Surreal};
use tokio::time::{sleep, Duration};
mod utils;
use utils::*;

lazy_static! {
    static ref WIKIDATA_FILE_FORMAT: String =
        env::var("WIKIDATA_FILE_FORMAT").expect("FILE_FORMAT not set");
    static ref WIKIDATA_FILE_NAME: String =
        env::var("WIKIDATA_FILE_NAME").expect("FILE_NAME not set");
    static ref THREADED_REQUESTS: bool = env::var("THREADED_REQUESTS")
        .expect("THREADED_REQUESTS not set")
        .parse()
        .expect("Failed to parse THREADED_REQUESTS");
    static ref WIKIDATA_BULK_INSERT: bool = env::var("WIKIDATA_BULK_INSERT")
        .expect("WIKIDATA_BULK_INSERT not set")
        .parse()
        .expect("Failed to parse WIKIDATA_BULK_INSERT");
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    sleep(Duration::from_secs(10)).await;
    let pb = create_pb().await;

    let db = create_db_ws().await?;
    let reader = File_Format::new(&WIKIDATA_FILE_FORMAT).reader(&WIKIDATA_FILE_NAME)?;

    if !*THREADED_REQUESTS {
        let mut counter = 0;
        for line in reader.lines() {
            let mut retries = 0;
            let line = line?;

            loop {
                if create_db_entity(&db, &line).await.is_ok() {
                    break;
                }
                if retries >= 60 * 10 {
                    panic!("Failed to create entities, too many retries");
                }
                retries += 1;
                sleep(Duration::from_secs(1)).await;
                if db.use_ns("wikidata").use_db("wikidata").await.is_err() {
                    continue;
                };
            }

            counter += 1;
            if counter % 100 == 0 {
                pb.inc(100);
            }
        }
    } else if *WIKIDATA_BULK_INSERT {
        create_db_entities_threaded(
            None::<Surreal<Client>>,
            reader,
            Some(pb.clone()),
            2500,
            100,
            CreateVersion::Bulk,
        )
        .await?;
    } else {
        create_db_entities_threaded(
            None::<Surreal<Client>>,
            reader,
            Some(pb.clone()),
            2500,
            100,
            CreateVersion::Single,
        )
        .await?;
    }

    pb.finish();
    Ok(())
}
