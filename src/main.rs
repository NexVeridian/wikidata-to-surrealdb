use anyhow::{Error, Ok, Result};
use bzip2::read::MultiBzDecoder;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use lazy_static::lazy_static;
use serde_json::{from_str, Value};
use std::{
    env,
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
    thread,
    time::Duration,
};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use wikidata::Entity;

mod utils;
use utils::*;

lazy_static! {
    #[derive(Debug)]
    static ref DB_USER: String = env::var("DB_USER").expect("DB_USER not set");
    static ref DB_PASSWORD: String = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
    static ref WIKIDATA_FILE_FORMAT: String = env::var("WIKIDATA_FILE_FORMAT").expect("FILE_FORMAT not set");
    static ref WIKIDATA_FILE_NAME: String = env::var("WIKIDATA_FILE_NAME").expect("FILE_NAME not set");
    static ref WIKIDATA_DB_PORT: String = env::var("WIKIDATA_DB_PORT").expect("WIKIDATA_DB_PORT not set");
}

#[allow(non_camel_case_types)]
enum File_Format {
    json,
    bz2,
}
impl File_Format {
    fn new(file: &str) -> Self {
        match file {
            "json" => Self::json,
            "bz2" => Self::bz2,
            _ => panic!("Unknown file format"),
        }
    }
    fn reader(self, file: &str) -> Result<Box<dyn BufRead>, Error> {
        let file = File::open(file)?;
        match self {
            File_Format::json => Ok(Box::new(BufReader::new(file))),
            File_Format::bz2 => Ok(Box::new(BufReader::new(MultiBzDecoder::new(file)))),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    thread::sleep(Duration::from_secs(10));

    let mut compleated = 0;
    let total_size = 113_000_000;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} {percent} ETA:{eta}",
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

    for line in reader.lines() {
        let line = line?.trim().trim_end_matches(',').to_string();
        if line == "[" || line == "]" {
            continue;
        }

        let json: Value = from_str(&line)?;
        let data = Entity::from_json(json).expect("Failed to parse JSON");

        let (mut claims, mut data) = EntityMini::from_entity(data);

        let id = data.id.clone().expect("No ID");
        data.id = None;
        let _: Option<EntityMini> = db.delete(&id).await?;
        let _: Option<EntityMini> = db.create(&id).content(data.clone()).await?;

        let id = claims.id.clone().expect("No ID");
        claims.id = None;
        let _: Option<Claims> = db.delete(&id).await?;
        let _: Option<Claims> = db.create(&id).content(claims).await?;

        compleated += 1;
        if compleated % 1000 == 0 {
            pb.set_position(compleated);
        }
    }

    pb.finish_with_message("Done parsing Wikidata");
    Ok(())
}
