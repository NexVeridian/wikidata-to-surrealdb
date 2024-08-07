use anyhow::{Error, Ok, Result};
use std::{env, io::BufRead};
use surrealdb::{
    engine::local::{Db, Mem},
    Surreal,
};

use wikidata_to_surrealdb::utils::*;

async fn inti_db() -> Result<Surreal<Db>, Error> {
    env::set_var("WIKIDATA_LANG", "en");
    env::set_var("OVERWRITE_DB", "true");

    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    Ok(db)
}

fn init_reader(file_format: &str, file_name: &str) -> Box<dyn BufRead> {
    File_Format::new(file_format)
        .reader(&format!("./tests/data/{}.{}", file_name, file_format))
        .unwrap()
}

async fn entity_query(db: &Surreal<Db>) -> Result<Option<f32>, Error> {
    let x: Option<f32> = db
        .query(r#"
		return
		(
		select claims.claims[where id = Property:1113][0].value.ClaimValueData.Quantity.amount as number_of_episodes from Entity
		where label = "Black Clover, season 1"
		)[0].number_of_episodes;
		"#)
        .await
        .unwrap()
        .take(0)
        .unwrap();
    Ok(x)
}

#[tokio::test]
async fn entity() {
    let db = inti_db().await.unwrap();
    let reader = init_reader("json", "Entity");

    for line in reader.lines() {
        create_db_entity(&db, &line.unwrap()).await.unwrap();
    }

    assert_eq!(51.0, entity_query(&db).await.unwrap().unwrap())
}

async fn entity_threaded_insert(create_version: CreateVersion) -> Result<Surreal<Db>, Error> {
    let db = inti_db().await?;
    let reader = File_Format::new("json").reader("tests/data/Entity.json")?;

    create_db_entities_threaded(Some(db.clone()), reader, None, 1000, 100, create_version).await?;
    Ok(db)
}

#[tokio::test]
async fn entity_threaded() {
    let db = entity_threaded_insert(CreateVersion::Single).await.unwrap();
    assert_eq!(51.0, entity_query(&db).await.unwrap().unwrap())
}

#[tokio::test]
async fn entity_threaded_bulk_insert() {
    let db = entity_threaded_insert(CreateVersion::Bulk).await.unwrap();
    assert_eq!(51.0, entity_query(&db).await.unwrap().unwrap())
}

async fn property_query(db: &Surreal<Db>) -> Result<Option<f32>, Error> {
    let x: Option<f32> = db
        .query("return count(select * from Property);")
        .await
        .unwrap()
        .take(0)
        .unwrap();
    Ok(x)
}

#[tokio::test]
async fn property() {
    let db = inti_db().await.unwrap();
    let reader = init_reader("json", "Property");

    for line in reader.lines() {
        create_db_entity(&db, &line.unwrap()).await.unwrap();
    }

    assert_eq!(2.0, property_query(&db).await.unwrap().unwrap())
}

async fn property_threaded_insert(create_version: CreateVersion) -> Result<Surreal<Db>, Error> {
    let db = inti_db().await?;
    let reader = init_reader("json", "Property");

    create_db_entities_threaded(Some(db.clone()), reader, None, 1000, 100, create_version).await?;
    Ok(db)
}

#[tokio::test]
async fn property_threaded_single_insert() {
    let db = property_threaded_insert(CreateVersion::Single)
        .await
        .unwrap();
    assert_eq!(2.0, property_query(&db).await.unwrap().unwrap())
}

#[tokio::test]
async fn property_threaded_bulk_insert() {
    let db = property_threaded_insert(CreateVersion::Bulk).await.unwrap();
    assert_eq!(2.0, property_query(&db).await.unwrap().unwrap())
}
