use anyhow::{Error, Ok, Result};
use std::{env, io::BufRead};
use surrealdb::{
    engine::local::{Db, Mem},
    Surreal,
};

use wikidata_to_surrealdb::utils::*;

async fn inti_db() -> Result<Surreal<Db>, Error> {
    env::set_var("WIKIDATA_LANG", "en");

    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("wikidata").use_db("wikidata").await?;

    Ok(db)
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
    let reader = File_Format::new("json")
        .reader("tests/data/Entity.json")
        .unwrap();

    for line in reader.lines() {
        create_db_entity(&db, line.unwrap()).await.unwrap();
    }

    assert_eq!(51.0, entity_query(&db).await.unwrap().unwrap())
}

#[tokio::test]
async fn entity_threaded() {
    let db = inti_db().await.unwrap();
    let reader = File_Format::new("json")
        .reader("tests/data/Entity.json")
        .unwrap();

    create_db_entities_threaded(&db, reader, None, 1000, 100)
        .await
        .unwrap();

    assert_eq!(51.0, entity_query(&db).await.unwrap().unwrap())
}

async fn property_query(db: &Surreal<Db>) -> Result<Option<f32>, Error> {
    let x: Option<f32> = db
        .query(
            r#"
		return count(select * from Property);
		"#,
        )
        .await
        .unwrap()
        .take(0)
        .unwrap();
    Ok(x)
}

#[tokio::test]
async fn property() {
    let db = inti_db().await.unwrap();
    let reader = File_Format::new("json")
        .reader("tests/data/Property.json")
        .unwrap();

    for line in reader.lines() {
        create_db_entity(&db, line.unwrap()).await.unwrap();
    }

    assert_eq!(2.0, property_query(&db).await.unwrap().unwrap())
}

#[tokio::test]
async fn property_threaded() {
    let db = inti_db().await.unwrap();
    let reader = File_Format::new("json")
        .reader("tests/data/Property.json")
        .unwrap();

    create_db_entities_threaded(&db, reader, None, 1000, 100)
        .await
        .unwrap();

    assert_eq!(2.0, property_query(&db).await.unwrap().unwrap())
}
