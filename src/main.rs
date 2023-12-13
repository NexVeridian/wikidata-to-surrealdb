use anyhow::{Error, Result};
use dotenv_codegen::dotenv;
use serde_json::{from_str, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

mod utils;
use utils::*;
use wikidata::Entity;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Surreal::new::<Ws>("0.0.0.0:8000").await?;

    db.signin(Root {
        username: dotenv!("DB_USER"),
        password: dotenv!("DB_PASSWORD"),
    })
    .await?;

    db.use_ns("wikidata").use_db("wikidata").await?;

    let file = File::open("data/w.json")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?.trim().trim_end_matches(',').to_string();
        if line == "[" || line == "]" {
            continue;
        }

        let json: Value = from_str(&line)?;
        let data = Entity::from_json(json).expect("Failed to parse JSON");

        let (id, data) = EntityMini::from_entity(data);

        let _: Option<EntityMini> = db.delete(&id).await?;
        let _: Option<EntityMini> = db.create(&id).content(data.clone()).await?;
    }

    Ok(())
}
