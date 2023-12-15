use anyhow::{Error, Ok, Result};
use bzip2::read::MultiBzDecoder;
use dotenv_codegen::dotenv;
use serde_json::{from_str, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use wikidata::Entity;

mod utils;
use utils::*;

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
        username: dotenv!("DB_USER"),
        password: dotenv!("DB_PASSWORD"),
    })
    .await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    let reader = File_Format::new(dotenv!("FILE_FORMAT")).reader(dotenv!("FILE_NAME"))?;

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
