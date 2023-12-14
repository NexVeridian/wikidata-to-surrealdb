use anyhow::{Error, Ok, Result};
use dotenv_codegen::dotenv;
use serde_json::{from_str, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use wikidata::Entity;

mod utils;
use utils::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Surreal::new::<Ws>("0.0.0.0:8000").await?;

    db.signin(Root {
        username: dotenv!("DB_USER"),
        password: dotenv!("DB_PASSWORD"),
    })
    .await?;

    db.use_ns("wikidata").use_db("wikidata").await?;

    let file = File::open("data/ex2.json")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?.trim().trim_end_matches(',').to_string();
        if line == "[" || line == "]" {
            continue;
        }

        let json: Value = from_str(&line)?;
        let data = Entity::from_json(json).expect("Failed to parse JSON");

        let (id, claims, data) = EntityMini::from_entity(data);

        let _: Option<EntityMini> = db.delete(&id).await?;
        let _: Option<EntityMini> = db.create(&id).content(data.clone()).await?;

        let _: Option<Claims> = db.delete(&claims.0).await?;
        let _: Option<Claims> = db.create(&claims.0).content(claims.1).await?;
    }

    Ok(())
}
