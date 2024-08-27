use anyhow::{Error, Result};
use bzip2::read::MultiBzDecoder;
use core::panic;
use futures::future::join_all;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{from_str, Value};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
};
use surrealdb::{Connection, Surreal};
use tokio::time::{sleep, Duration};
use wikidata::Entity;

pub mod init_db;
pub mod init_progress_bar;
mod tables;
use tables::*;

lazy_static! {
    static ref OVERWRITE_DB: bool = env::var("OVERWRITE_DB")
        .expect("OVERWRITE_DB not set")
        .parse()
        .expect("Failed to parse OVERWRITE_DB");
    static ref FILTER_PATH: String =
        env::var("FILTER_PATH").unwrap_or("../filter.surql".to_string());
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
    lines: &[String],
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

pub async fn create_db_entities_bulk(
    db: &Surreal<impl Connection>,
    lines: &[String],
    pb: &Option<ProgressBar>,
    batch_size: usize,
) -> Result<(), Error> {
    let lines = lines
        .iter()
        .map(|line| line.trim().trim_end_matches(',').to_string())
        .filter(|line| line != "[" && line != "]")
        .collect::<Vec<String>>();

    let mut data_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);
    let mut claims_vec: Vec<Claims> = Vec::with_capacity(batch_size);
    let mut property_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);
    let mut lexeme_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);

    for line in lines {
        let json: Value = from_str(&line).expect("Failed to parse JSON");
        let data = Entity::from_json(json).expect("Failed to parse JSON");
        let (claims, data) = EntityMini::from_entity(data);
        match data.id.clone().expect("No ID").tb.as_str() {
            "Property" => property_vec.push(data),
            "Lexeme" => lexeme_vec.push(data),
            "Entity" => data_vec.push(data),
            _ => panic!("Unknown table"),
        }
        claims_vec.push(claims);
    }

    db.insert::<Vec<EntityMini>>("Entity")
        .content(data_vec)
        .await?;
    db.insert::<Vec<Claims>>("Claims")
        .content(claims_vec)
        .await?;
    db.insert::<Vec<EntityMini>>("Property")
        .content(property_vec)
        .await?;
    db.insert::<Vec<EntityMini>>("Lexeme")
        .content(lexeme_vec)
        .await?;

    if let Some(ref p) = pb {
        p.inc(batch_size as u64)
    }
    Ok(())
}

pub async fn create_db_entities_bulk_filter(
    db: &Surreal<impl Connection>,
    lines: &[String],
    pb: &Option<ProgressBar>,
    batch_size: usize,
) -> Result<(), Error> {
    let db_mem = init_db::create_db_mem().await?;
    create_db_entities_bulk(&db_mem, lines, &None, batch_size).await?;

    let filter = tokio::fs::read_to_string(&*FILTER_PATH).await?;
    db_mem.query(filter).await?;

    let file_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    let file_path = format!("data/temp/{}.surql", file_name);
    tokio::fs::create_dir_all("data/temp").await?;

    db_mem.export(&file_path).await?;
    db.import(&file_path).await?;

    tokio::fs::remove_file(&file_path).await?;

    if let Some(ref p) = pb {
        p.inc(batch_size as u64)
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub enum CreateVersion {
    Single,
    Bulk,
    // must create a filter.surql file in the root directory
    BulkFilter,
}
impl CreateVersion {
    pub async fn run(
        self,
        db: &Surreal<impl Connection>,
        chunk: &[String],
        pb: &Option<ProgressBar>,
        batch_size: usize,
    ) -> bool {
        match self {
            CreateVersion::Single => create_db_entities(db, chunk, pb).await.is_ok(),
            CreateVersion::Bulk => create_db_entities_bulk(db, chunk, pb, batch_size)
                .await
                .is_ok(),
            CreateVersion::BulkFilter => create_db_entities_bulk_filter(db, chunk, pb, batch_size)
                .await
                .is_ok(),
        }
    }
}

pub async fn create_db_entities_threaded(
    dbo: Option<Surreal<impl Connection>>, // None::<Surreal<Client>>
    reader: Box<dyn BufRead>,
    pb: Option<ProgressBar>,
    batch_size: usize,
    batch_num: usize,
    create_version: CreateVersion,
) -> Result<(), Error> {
    let mut futures = Vec::new();
    let mut chunk = Vec::with_capacity(batch_size);
    let mut chunk_counter = 0;
    let mut lines = reader.lines();
    let mut last_loop = false;

    loop {
        let line = lines.next();
        match line {
            Some(line) => chunk.push(line?),
            None => last_loop = true,
        };

        if chunk.len() >= batch_size || last_loop {
            let dbo = dbo.clone();
            let pb = pb.clone();

            futures.push(tokio::spawn(async move {
                let mut retries = 0;
                loop {
                    match dbo {
                        Some(ref db) => {
                            if create_version.run(db, &chunk, &pb, batch_size).await {
                                break;
                            }
                            if db.use_ns("wikidata").use_db("wikidata").await.is_err() {
                                continue;
                            };
                        }
                        None => {
                            let db = if let Ok(db) = init_db::create_db_ws().await {
                                db
                            } else {
                                continue;
                            };
                            if create_version.run(&db, &chunk, &pb, batch_size).await {
                                break;
                            }
                        }
                    }

                    if retries >= 60 * 10 {
                        panic!("Failed to create entities, too many retries");
                    }
                    retries += 1;
                    sleep(Duration::from_millis(250)).await;
                }
            }));
            chunk_counter += 1;
            chunk = Vec::with_capacity(batch_size);
        }

        if chunk_counter >= batch_num || last_loop {
            join_all(futures).await;
            futures = Vec::new();
            chunk_counter = 0;
        }
        if last_loop {
            break;
        }
    }

    match dbo {
        Some(db) => {
            create_db_entities(&db, &chunk, &pb).await?;
        }
        None => {
            create_db_entities(&init_db::create_db_ws().await?, &chunk, &pb).await?;
        }
    }
    join_all(futures).await;
    Ok(())
}
