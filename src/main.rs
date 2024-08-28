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
    static ref CREATE_MODE: CreateMode = match env::var("CREATE_MODE")
        .expect("CREATE_MODE not set")
        .as_str()
    {
        "Single" => CreateMode::Single,
        "ThreadedSingle" => CreateMode::ThreadedSingle,
        "ThreadedBulk" => CreateMode::ThreadedBulk,
        "ThreadedBulkFilter" => CreateMode::ThreadedBulkFilter,
        _ => panic!("Unknown CREATE_MODE"),
    };
}

#[derive(Clone, Copy)]
pub enum CreateMode {
    Single,
    ThreadedSingle,
    ThreadedBulk,
    // must create a filter.surql file in the root directory
    ThreadedBulkFilter,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    sleep(Duration::from_secs(10)).await;
    let pb = init_progress_bar::create_pb().await;

    let db = init_db::create_db_ws().await?;
    let reader = File_Format::new(&WIKIDATA_FILE_FORMAT).reader(&WIKIDATA_FILE_NAME)?;

    match *CREATE_MODE {
        CreateMode::Single => {
            let mut counter = 0;
            for line in reader.lines() {
                let mut retries = 0;
                let line = line?;

                loop {
                    if create_entity(&db, &line).await.is_ok() {
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
        }
        CreateMode::ThreadedSingle => {
            CreateVersion::Single
                .run_threaded(
                    None::<Surreal<Client>>,
                    reader,
                    Some(pb.clone()),
                    2_500,
                    100,
                )
                .await?;
        }
        CreateMode::ThreadedBulk => {
            CreateVersion::Bulk
                .run_threaded(
                    None::<Surreal<Client>>,
                    reader,
                    Some(pb.clone()),
                    500,
                    1_000,
                )
                .await?;
        }
        CreateMode::ThreadedBulkFilter => {
            CreateVersion::BulkFilter
                .run_threaded(
                    None::<Surreal<Client>>,
                    reader,
                    Some(pb.clone()),
                    500,
                    1_000,
                )
                .await?;
        }
    }

    pb.finish();
    Ok(())
}
