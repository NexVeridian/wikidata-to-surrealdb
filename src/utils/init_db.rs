use anyhow::Error;
use anyhow::Result;
use lazy_static::lazy_static;
use std::env;
use surrealdb::{
    engine::{
        local::{Db, Mem},
        remote::http::{Client, Http},
    },
    opt::auth::Root,
    Surreal,
};

lazy_static! {
    static ref DB_USER: String = env::var("DB_USER").expect("DB_USER not set");
    static ref DB_PASSWORD: String = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
    static ref WIKIDATA_DB_PORT: String =
        env::var("WIKIDATA_DB_PORT").expect("WIKIDATA_DB_PORT not set");
}

pub async fn create_db_remote() -> Result<Surreal<Client>, Error> {
    let db = Surreal::new::<Http>(WIKIDATA_DB_PORT.as_str()).await?;

    db.signin(Root {
        username: &DB_USER,
        password: &DB_PASSWORD,
    })
    .await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    Ok(db)
}

pub async fn create_db_mem() -> Result<Surreal<Db>, Error> {
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    Ok(db)
}
