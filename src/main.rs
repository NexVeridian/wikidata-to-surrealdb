use anyhow::Ok;
use anyhow::{Error, Result};
use dotenv_codegen::dotenv;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

mod utils;
use utils::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let data = get_entity("data/e.json").await?;
    let (id, data) = EntityMini::from_entity(data);

    let db = Surreal::new::<Ws>("0.0.0.0:8000").await?;

    db.signin(Root {
        username: dotenv!("DB_USER"),
        password: dotenv!("DB_PASSWORD"),
    })
    .await?;

    db.use_ns("wikidata").use_db("wikidata").await?;

    let _: Option<EntityMini> = db.delete(id.to_string()).await?;
    let _: Option<EntityMini> = db.create(id.to_string()).content(data.clone()).await?;

    // println!("{:#?}", data);
    Ok(())
}
