use anyhow::{Error, Result};
use bzip2::read::MultiBzDecoder;
use futures::future::join_all;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use serde_json::{from_str, Value};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Connection, Surreal,
};
use tokio::time::{sleep, Duration};
use wikidata::Entity;

mod tables;
use tables::*;

lazy_static! {
    static ref OVERWRITE_DB: bool = env::var("OVERWRITE_DB")
        .expect("OVERWRITE_DB not set")
        .parse()
        .expect("Failed to parse OVERWRITE_DB");
    static ref DB_USER: String = env::var("DB_USER").expect("DB_USER not set");
    static ref DB_PASSWORD: String = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
    static ref WIKIDATA_DB_PORT: String =
        env::var("WIKIDATA_DB_PORT").expect("WIKIDATA_DB_PORT not set");
}

#[allow(non_camel_case_types)]
pub enum File_Format {
    json,
    bz2,
}
impl File_Format {
    pub fn new(file: &str) -> Self {
        match file {
            "json" => Self::json,
            "bz2" => Self::bz2,
            _ => panic!("Unknown file format"),
        }
    }
    pub fn reader(self, file: &str) -> Result<Box<dyn BufRead>, Error> {
        let file = File::open(file)?;
        match self {
            File_Format::json => Ok(Box::new(BufReader::new(file))),
            File_Format::bz2 => Ok(Box::new(BufReader::new(MultiBzDecoder::new(file)))),
        }
    }
}

pub async fn create_db_entity(db: &Surreal<impl Connection>, line: &str) -> Result<(), Error> {
    let line = line.trim().trim_end_matches(',').to_string();
    if line == "[" || line == "]" {
        return Ok(());
    }

    let json: Value = from_str(&line)?;
    let data = Entity::from_json(json).expect("Failed to parse JSON");

    let (mut claims, mut data) = EntityMini::from_entity(data);

    let id = data.id.clone().expect("No ID");
    data.id = None;
    if db.create::<Option<EntityMini>>(&id).await.is_err() && *OVERWRITE_DB {
        db.update::<Option<EntityMini>>(&id).content(data).await?;
    }

    let id = claims.id.clone().expect("No ID");
    claims.id = None;
    if db.create::<Option<Claims>>(&id).await.is_err() && *OVERWRITE_DB {
        db.update::<Option<Claims>>(&id).content(claims).await?;
    }
    Ok(())
}

pub async fn create_db_entities(
    db: &Surreal<impl Connection>,
    lines: &Vec<String>,
    pb: &Option<ProgressBar>,
) -> Result<(), Error> {
    let mut counter = 0;
    for line in lines {
        create_db_entity(db, line).await?;
        counter += 1;
        if counter % 100 == 0 {
            if let Some(ref p) = pb {
                p.inc(100)
            }
        }
    }
    Ok(())
}

pub async fn create_db_entities_threaded(
    dbo: Option<Surreal<impl Connection>>, // None::<Surreal<Client>>
    reader: Box<dyn BufRead>,
    pb: Option<ProgressBar>,
    batch_size: usize,
    batch_num: usize,
) -> Result<(), Error> {
    let mut futures = Vec::new();
    let mut chunk = Vec::new();
    let mut chunk_counter = 0;

    for line in reader.lines() {
        chunk.push(line?);

        if chunk.len() >= batch_size {
            let dbo = dbo.clone();
            let pb = pb.clone();

            futures.push(tokio::spawn(async move {
                let mut retries = 0;
                loop {
                    match dbo {
                        Some(ref db) => {
                            if create_db_entities(db, &chunk, &pb).await.is_ok() {
                                break;
                            }
                            if db.use_ns("wikidata").use_db("wikidata").await.is_err() {
                                continue;
                            };
                        }
                        None => {
                            let db = if let Ok(db) = create_db_ws().await {
                                db
                            } else {
                                continue;
                            };
                            if create_db_entities(&db, &chunk, &pb).await.is_ok() {
                                break;
                            }
                        }
                    }

                    if retries >= 60 * 10 {
                        panic!("Failed to create entities, too many retries");
                    }
                    retries += 1;
                    sleep(Duration::from_secs(1)).await;
                }
            }));
            chunk_counter += 1;
            chunk = Vec::new();
        }

        if chunk_counter >= batch_num {
            join_all(futures).await;
            futures = Vec::new();
            chunk_counter = 0;
        }
    }

    match dbo {
        Some(db) => {
            create_db_entities(&db, &chunk, &pb).await?;
        }
        None => {
            create_db_entities(&create_db_ws().await?, &chunk, &pb).await?;
        }
    }
    join_all(futures).await;
    Ok(())
}

pub async fn create_db_ws() -> Result<Surreal<Client>, Error> {
    let db = Surreal::new::<Ws>(WIKIDATA_DB_PORT.as_str()).await?;

    db.signin(Root {
        username: &DB_USER,
        password: &DB_PASSWORD,
    })
    .await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    Ok(db)
}
