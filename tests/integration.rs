use anyhow::{Error, Ok, Result};
use rstest::rstest;
use std::{env, io::BufRead};
use surrealdb::{Surreal, engine::local::Db};

use init_reader::File_Format;
use wikidata_to_surrealdb::utils::*;

async fn inti_db() -> Result<Surreal<Db>, Error> {
    unsafe { env::set_var("WIKIDATA_LANG", "en") };

    let db = init_db::create_db_mem().await?;

    Ok(db)
}

async fn init_reader(file_format: &str, file_name: &str) -> Box<dyn BufRead> {
    File_Format::new(file_format)
        .await
        .reader(&format!("./tests/data/{}.{}", file_name, file_format))
        .await
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

#[rstest]
#[case(CreateVersion::Bulk)]
#[tokio::test]
async fn entity_threaded(#[case] version: CreateVersion) -> Result<(), Error> {
    let db = inti_db().await?;
    let reader = init_reader("json", "Entity").await;

    version
        .run(Some(db.clone()), reader, None, 1_000, 100)
        .await?;

    assert_eq!(51.0, entity_query(&db).await?.unwrap());
    Ok(())
}

#[tokio::test]
async fn entity_threaded_filter() -> Result<(), Error> {
    unsafe { env::set_var("FILTER_PATH", "./tests/data/test_filter.surql") };
    let db = inti_db().await?;
    let reader = init_reader("json", "bench").await;

    CreateVersion::BulkFilter
        .run(Some(db.clone()), reader, None, 1_000, 100)
        .await?;

    let count: Option<f32> = db
        .query("return count(select * from Entity);")
        .await
        .unwrap()
        .take(0)
        .unwrap();

    assert_eq!(3.0, count.unwrap());
    Ok(())
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

#[rstest]
#[case(CreateVersion::Bulk)]
#[tokio::test]
async fn property_threaded(#[case] version: CreateVersion) -> Result<(), Error> {
    let db = inti_db().await?;
    let reader = init_reader("json", "Property").await;

    version
        .run(Some(db.clone()), reader, None, 1_000, 100)
        .await?;

    assert_eq!(2.0, property_query(&db).await?.unwrap());
    Ok(())
}
