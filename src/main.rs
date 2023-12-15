use anyhow::{Error, Ok, Result};
use bzip2::read::MultiBzDecoder;
use lazy_static::lazy_static;
use serde_json::{from_str, Value};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use wikidata::Entity;

mod utils;
use utils::*;

lazy_static! {
    #[derive(Debug)]
    static ref DB_USER: String = env::var("DB_USER").expect("DB_USER not set");
    static ref DB_PASSWORD: String = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
    static ref FILE_FORMAT: String = env::var("FILE_FORMAT").expect("FILE_FORMAT not set");
    static ref FILE_NAME: String = env::var("FILE_NAME").expect("FILE_NAME not set");
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
    let db = Surreal::new::<Ws>("0.0.0.0:8000").await?;
    db.signin(Root {
        username: &DB_USER,
        password: &DB_PASSWORD,
    })
    .await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    let reader = File_Format::new(&FILE_FORMAT).reader(&FILE_NAME)?;

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
    }

    Ok(())
}
