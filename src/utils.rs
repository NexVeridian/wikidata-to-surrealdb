use anyhow::{Error, Result};
use backon::Retryable;
use core::panic;
use futures::future::join_all;
use indicatif::ProgressBar;
use rand::{Rng, distr::Alphanumeric};
use serde_json::{Value, from_str};
use std::{env, io::BufRead};
use surrealdb::{Connection, Surreal};
use tokio::sync::OnceCell;
use wikidata::Entity;

pub mod init_backoff;
pub mod init_db;
pub mod init_progress_bar;
pub mod init_reader;
mod tables;
use tables::*;

static FILTER_PATH: OnceCell<String> = OnceCell::const_new();

async fn get_filter_path() -> &'static String {
    FILTER_PATH
        .get_or_init(|| async {
            env::var("FILTER_PATH").unwrap_or("data/filter.surql".to_string())
        })
        .await
}

#[derive(Clone, Copy, Default)]
pub enum CreateVersion {
    #[default]
    Bulk,
    /// must create a `filter.surql` file in the root directory
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
        self,
        dbo: Option<Surreal<impl Connection>>,
        chunk: Vec<String>,
        pb: Option<ProgressBar>,
        batch_size: usize,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            match dbo {
                Some(db) => self.create_retry(&db, &chunk, &pb, batch_size).await,
                None => {
                    let db = init_db::create_db_remote
                        .retry(*init_backoff::get_exponential().await)
                        .await
                        .expect("Failed to create remote db");
                    self.create_retry(&db, &chunk, &pb, batch_size).await
                }
            }
            .unwrap_or_else(|err| panic!("Failed to create entities, too many retries: {}", err));
        })
    }

    /// Retry create with exponential backoff
    async fn create_retry(
        self,
        db: &Surreal<impl Connection>,
        chunk: &[String],
        pb: &Option<ProgressBar>,
        batch_size: usize,
    ) -> Result<(), Error> {
        (|| async { self.create(db, chunk, pb, batch_size).await })
            .retry(*init_backoff::get_exponential().await)
            .await
    }

    async fn create(
        self,
        db: &Surreal<impl Connection>,
        chunk: &[String],
        pb: &Option<ProgressBar>,
        batch_size: usize,
    ) -> Result<(), Error> {
        match self {
            Self::Bulk => self.create_bulk(db, chunk, pb, batch_size).await,
            Self::BulkFilter => self.create_bulk_filter(db, chunk, pb, batch_size).await,
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
            let (claims, data) = EntityMini::from_entity(data).await;
            match data.id.clone().expect("No ID").tb.as_str() {
                "Property" => property_vec.push(data),
                "Lexeme" => lexeme_vec.push(data),
                "Entity" => entity_vec.push(data),
                _ => continue,
            }
            claims_vec.push(claims);
        }

        db.query("INSERT INTO Entity $entity_vec RETURN NONE;")
            .bind(("entity_vec", entity_vec))
            .query("INSERT INTO Claims $claims_vec RETURN NONE;")
            .bind(("claims_vec", claims_vec))
            .query("INSERT INTO Property $property_vec RETURN NONE;")
            .bind(("property_vec", property_vec))
            .query("INSERT INTO Lexeme $lexeme_vec RETURN NONE;")
            .bind(("lexeme_vec", lexeme_vec))
            .await?;

        if let Some(p) = pb {
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

        let filter = tokio::fs::read_to_string(get_filter_path().await).await?;
        db_mem.query(filter).await?;

        let file_name: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        let file_path = format!("data/temp/{file_name}.surql");

        tokio::fs::create_dir_all("data/temp").await?;

        db_mem.export(&file_path).await?;
        db.import(&file_path).await?;

        tokio::fs::remove_file(&file_path).await?;

        if let Some(p) = pb {
            p.inc(batch_size as u64)
        }
        Ok(())
    }
}
