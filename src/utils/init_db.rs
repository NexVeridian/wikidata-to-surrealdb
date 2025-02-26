use anyhow::Error;
use anyhow::Result;
use std::env;
use surrealdb::{
    Surreal,
    engine::{
        local::{Db, Mem},
        remote::http::{Client, Http},
    },
    opt::auth::Root,
};
use tokio::sync::OnceCell;

static DB_USER: OnceCell<String> = OnceCell::const_new();
static DB_PASSWORD: OnceCell<String> = OnceCell::const_new();
static WIKIDATA_DB_PORT: OnceCell<String> = OnceCell::const_new();

pub async fn get_db_user() -> &'static String {
    DB_USER
        .get_or_init(|| async { env::var("DB_USER").expect("DB_USER not set") })
        .await
}

pub async fn get_db_password() -> &'static String {
    DB_PASSWORD
        .get_or_init(|| async { env::var("DB_PASSWORD").expect("DB_PASSWORD not set") })
        .await
}

pub async fn get_wikidata_db_port() -> &'static String {
    WIKIDATA_DB_PORT
        .get_or_init(|| async { env::var("WIKIDATA_DB_PORT").expect("WIKIDATA_DB_PORT not set") })
        .await
}

pub async fn create_db_remote() -> Result<Surreal<Client>, Error> {
    let db = Surreal::new::<Http>(get_wikidata_db_port().await).await?;

    db.signin(Root {
        username: get_db_user().await,
        password: get_db_password().await,
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
