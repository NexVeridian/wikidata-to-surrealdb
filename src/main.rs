use anyhow::{Error, Ok, Result};
use std::env;
use surrealdb::{engine::remote::http::Client, Surreal};
use tokio::{
    fs,
    sync::OnceCell,
    time::{sleep, Duration},
};
mod utils;
use init_reader::File_Format;
use utils::*;

static WIKIDATA_FILE_FORMAT: OnceCell<String> = OnceCell::const_new();
static WIKIDATA_FILE_NAME: OnceCell<String> = OnceCell::const_new();
static CREATE_VERSION: OnceCell<CreateVersion> = OnceCell::const_new();

async fn get_wikidata_file_format() -> &'static String {
    WIKIDATA_FILE_FORMAT
        .get_or_init(|| async { env::var("WIKIDATA_FILE_FORMAT").expect("FILE_FORMAT not set") })
        .await
}

async fn get_wikidata_file_name() -> &'static String {
    WIKIDATA_FILE_NAME
        .get_or_init(|| async { env::var("WIKIDATA_FILE_NAME").expect("FILE_NAME not set") })
        .await
}

async fn get_create_version() -> &'static CreateVersion {
    CREATE_VERSION
        .get_or_init(|| async {
            match env::var("CREATE_VERSION")
                .expect("CREATE_VERSION not set")
                .as_str()
            {
                "Bulk" => CreateVersion::Bulk,
                "BulkFilter" => CreateVersion::BulkFilter,
                _ => panic!("Unknown CREATE_VERSION"),
            }
        })
        .await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    sleep(Duration::from_secs(10)).await;
    let pb = init_progress_bar::create_pb().await;
    let reader = File_Format::new(get_wikidata_file_format().await)
        .await
        .reader(get_wikidata_file_name().await)
        .await?;

    fs::create_dir_all("data/temp").await?;
    fs::remove_dir_all("data/temp").await?;
    fs::create_dir_all("data/temp").await?;

    get_create_version()
        .await
        .run(
            None::<Surreal<Client>>,
            reader,
            Some(pb.clone()),
            100,
            1_000,
        )
        .await?;

    pb.finish();
    Ok(())
}
