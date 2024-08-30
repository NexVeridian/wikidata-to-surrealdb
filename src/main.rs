use anyhow::{Error, Ok, Result};
use lazy_static::lazy_static;
use std::env;
use surrealdb::{engine::remote::http::Client, Surreal};
use tokio::time::{sleep, Duration};

mod utils;
use init_reader::File_Format;
use utils::*;

lazy_static! {
    static ref WIKIDATA_FILE_FORMAT: String =
        env::var("WIKIDATA_FILE_FORMAT").expect("FILE_FORMAT not set");
    static ref WIKIDATA_FILE_NAME: String =
        env::var("WIKIDATA_FILE_NAME").expect("FILE_NAME not set");
    static ref CREATE_VERSION: CreateVersion = match env::var("CREATE_VERSION")
        .expect("CREATE_VERSION not set")
        .as_str()
    {
        "Single" => CreateVersion::Single,
        "Bulk" => CreateVersion::Bulk,
        "BulkFilter" => CreateVersion::BulkFilter,
        _ => panic!("Unknown CREATE_VERSION"),
    };
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    sleep(Duration::from_secs(10)).await;
    let pb = init_progress_bar::create_pb().await;
    let reader = File_Format::new(&WIKIDATA_FILE_FORMAT).reader(&WIKIDATA_FILE_NAME)?;

    tokio::fs::create_dir_all("data/temp").await?;
    tokio::fs::remove_dir_all("data/temp").await?;
    tokio::fs::create_dir_all("data/temp").await?;

    CREATE_VERSION
        .run(
            None::<Surreal<Client>>,
            reader,
            Some(pb.clone()),
            1_000,
            1_000,
        )
        .await?;

    pb.finish();
    Ok(())
}
