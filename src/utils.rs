use anyhow::{Error, Ok, Result};
use bzip2::read::MultiBzDecoder;
use futures::future::join_all;
use indicatif::ProgressBar;
use serde_json::{from_str, Value};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use surrealdb::{Connection, Surreal};
use wikidata::Entity;

mod tables;
use tables::*;

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

pub async fn create_db_entity<C: Connection>(db: &Surreal<C>, line: String) -> Result<(), Error> {
    let line = line.trim().trim_end_matches(',').to_string();
    if line == "[" || line == "]" {
        return Ok(());
    }

    let json: Value = from_str(&line)?;
    let data = Entity::from_json(json).expect("Failed to parse JSON");

    let (mut claims, mut data) = EntityMini::from_entity(data);

    let id = data.id.clone().expect("No ID");
    data.id = None;
    let _ = db.create::<Option<EntityMini>>(&id).await.is_err();
    {
        db.update::<Option<EntityMini>>(&id).content(data).await?;
    };

    let id = claims.id.clone().expect("No ID");
    claims.id = None;
    let _ = db.create::<Option<Claims>>(&id).await.is_err();
    {
        db.update::<Option<Claims>>(&id).content(claims).await?;
    }
    Ok(())
}

pub async fn create_db_entities<C: Connection>(
    db: &Surreal<C>,
    lines: Vec<String>,
    pb: Option<ProgressBar>,
) -> Result<(), Error> {
    let mut counter = 0;
    for line in lines {
        create_db_entity(db, line.to_string()).await?;
        counter += 1;
        if counter % 100 == 0 {
            if let Some(ref p) = pb {
                p.inc(100)
            }
        }
    }
    Ok(())
}

pub async fn create_db_entities_threaded<C: Connection>(
    db: &Surreal<C>,
    reader: Box<dyn BufRead>,
    pb: Option<ProgressBar>,
    batch_size: usize,
    batch_num: usize,
) -> Result<(), Error> {
    let mut futures = Vec::new();
    let mut chunk = Vec::new();
    let mut chunk_counter = 0;

    for line in reader.lines() {
        chunk.push(line.unwrap());

        if chunk.len() >= batch_size {
            let db = db.clone();
            let lines = chunk.clone();
            let pb = pb.clone();

            futures.push(tokio::spawn(async move {
                create_db_entities(&db, lines, pb).await.unwrap();
            }));
            chunk_counter += 1;
            chunk.clear();
        }

        if chunk_counter >= batch_num {
            join_all(futures).await;
            futures = Vec::new();
            chunk_counter = 0;
        }
    }

    create_db_entities(db, chunk, pb).await.unwrap();
    join_all(futures).await;
    Ok(())
}
