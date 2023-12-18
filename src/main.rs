use anyhow::{Error, Ok, Result};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use lazy_static::lazy_static;
use std::{env, fmt::Write, io::BufRead};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use tokio::time::{sleep, Duration};
mod utils;
use utils::*;

lazy_static! {
    #[derive(Debug)]
    static ref DB_USER: String = env::var("DB_USER").expect("DB_USER not set");
    static ref DB_PASSWORD: String = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
    static ref WIKIDATA_FILE_FORMAT: String = env::var("WIKIDATA_FILE_FORMAT").expect("FILE_FORMAT not set");
    static ref WIKIDATA_FILE_NAME: String = env::var("WIKIDATA_FILE_NAME").expect("FILE_NAME not set");
    static ref WIKIDATA_DB_PORT: String = env::var("WIKIDATA_DB_PORT").expect("WIKIDATA_DB_PORT not set");
    static ref THREADED_REQUESTS: bool = env::var("THREADED_REQUESTS").expect("THREADED_REQUESTS not set").parse().expect("Failed to parse THREADED_REQUESTS");
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    sleep(Duration::from_secs(10)).await;
    let total_size = 113_000_000;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ETA:[{eta}]",
        )?
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
            let sec = state.eta().as_secs();
            let min = (sec / 60) % 60;
            let hr = (sec / 60) / 60;
            write!(w, "{}:{:02}:{:02}", hr, min, sec % 60).unwrap()
        }),
    );

    let db = Surreal::new::<Ws>(WIKIDATA_DB_PORT.as_str()).await?;

    db.signin(Root {
        username: &DB_USER,
        password: &DB_PASSWORD,
    })
    .await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

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
    } else {
        create_db_entities_threaded(&db, reader, Some(pb.clone()), 2500, 100).await?;
    }

    pb.finish();
    Ok(())
}
