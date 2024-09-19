use anyhow::{Error, Result};
use core::panic;
use futures::future::join_all;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{from_str, Value};
use std::{env, io::BufRead};
use surrealdb::{Connection, Surreal};
use tokio::time::{sleep, Duration};
use wikidata::Entity;

pub mod init_db;
pub mod init_progress_bar;
pub mod init_reader;
mod tables;
use tables::*;

lazy_static! {
    static ref OVERWRITE_DB: bool = env::var("OVERWRITE_DB")
        .expect("OVERWRITE_DB not set")
        .parse()
        .expect("Failed to parse OVERWRITE_DB");
    static ref FILTER_PATH: String =
        env::var("FILTER_PATH").unwrap_or("data/filter.surql".to_string());
}

#[derive(Clone, Copy, Default)]
pub enum CreateVersion {
    #[default]
    Bulk,
    /// must create a filter.surql file in the root directory
    BulkFilter,
}

impl CreateVersion {
    pub async fn run(
        self,
        dbo: Option<Surreal<impl Connection>>, // None::<Surreal<Client>>
        reader: Box<dyn BufRead>,
        pb: Option<ProgressBar>,
        batch_size: usize,
        batch_num: usize,
    ) -> Result<(), Error> {
        let mut lines = reader.lines().peekable();
        let mut futures = Vec::new();

        while lines.peek().is_some() {
            let chunk: Vec<String> = lines
                .by_ref()
                .take(batch_size)
                .filter_map(Result::ok)
                .collect();

            futures.push(self.spawn_chunk(dbo.clone(), chunk, pb.clone(), batch_size));

            if futures.len() >= batch_num {
                join_all(futures).await;
                futures = Vec::new();
            }
        }

        join_all(futures).await;
        Ok(())
    }

    fn spawn_chunk(
        &self,
        dbo: Option<Surreal<impl Connection>>,
        chunk: Vec<String>,
        pb: Option<ProgressBar>,
        batch_size: usize,
    ) -> tokio::task::JoinHandle<()> {
        let create_version = *self;

        tokio::spawn(async move {
            let mut retries = 0;

            loop {
                match dbo {
                    Some(ref db) => {
                        if create_version.create(db, &chunk, &pb, batch_size).await {
                            break;
                        }
                    }
                    None => {
                        let db = match init_db::create_db_remote().await {
                            Ok(db) => db,
                            Err(_) => continue,
                        };
                        if create_version.create(&db, &chunk, &pb, batch_size).await {
                            break;
                        }
                    }
                }

                // Exponential backoff with cap at 60 seconds
                if retries == 30 {
                    panic!("Failed to create entities, too many retries");
                }
                sleep(Duration::from_millis(250) * 2_u32.pow(retries.min(8))).await;
                retries += 1;
            }
        })
    }

    async fn create(
        self,
        db: &Surreal<impl Connection>,
        chunk: &[String],
        pb: &Option<ProgressBar>,
        batch_size: usize,
    ) -> bool {
        match self {
            CreateVersion::Bulk => self.create_bulk(db, chunk, pb, batch_size).await.is_ok(),
            CreateVersion::BulkFilter => self
                .create_bulk_filter(db, chunk, pb, batch_size)
                .await
                .is_ok(),
            // CreateVersion::BulkFilter => {
            //     if let Err(err) = self.create_bulk_filter(db, chunk, pb, batch_size).await {
            //         panic!("Failed to create entities: {}", err);
            //     }
            //     true
            // }
        }
    }

    async fn create_bulk(
        self,
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

        let mut entity_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);
        let mut claims_vec: Vec<Claims> = Vec::with_capacity(batch_size);
        let mut property_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);
        let mut lexeme_vec: Vec<EntityMini> = Vec::with_capacity(batch_size);

        for line in lines {
            let json: Value = from_str(&line).expect("Failed to parse JSON");
            let data = match Entity::from_json(json) {
                Ok(data) => data,
                Err(_) => continue,
            };
            let (claims, data) = EntityMini::from_entity(data);
            match data.id.clone().expect("No ID").tb.as_str() {
                "Property" => property_vec.push(data),
                "Lexeme" => lexeme_vec.push(data),
                "Entity" => entity_vec.push(data),
                _ => continue,
            }
            claims_vec.push(claims);
        }

        if *OVERWRITE_DB {
            db.upsert::<Vec<EntityMini>>("Entity")
                .content(entity_vec)
                .await?;
            db.upsert::<Vec<Claims>>("Claims")
                .content(claims_vec)
                .await?;
            db.upsert::<Vec<EntityMini>>("Property")
                .content(property_vec)
                .await?;
            db.upsert::<Vec<EntityMini>>("Lexeme")
                .content(lexeme_vec)
                .await?;
        } else {
            db.insert::<Vec<EntityMini>>("Entity")
                .content(entity_vec)
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
        }

        if let Some(ref p) = pb {
            p.inc(batch_size as u64)
        }
        Ok(())
    }

    async fn create_bulk_filter(
        self,
        db: &Surreal<impl Connection>,
        lines: &[String],
        pb: &Option<ProgressBar>,
        batch_size: usize,
    ) -> Result<(), Error> {
        let db_mem = init_db::create_db_mem().await?;
        self.create_bulk(&db_mem, lines, &None, batch_size).await?;

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
}
