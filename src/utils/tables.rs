use std::env;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Id, Thing};
use tokio::sync::OnceCell;
use wikidata::{ClaimValue, ClaimValueData, Entity, Lang, Pid, WikiId};

static WIKIDATA_LANG: OnceCell<String> = OnceCell::const_new();

async fn get_wikidata_lang() -> &'static String {
    WIKIDATA_LANG
        .get_or_init(|| async { env::var("WIKIDATA_LANG").expect("WIKIDATA_LANG not set") })
        .await
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClaimData {
    Thing(Thing),
    ClaimValueData(ClaimValueData),
}

impl ClaimData {
    async fn from_cvd(cvd: ClaimValueData) -> Self {
        match cvd {
            ClaimValueData::Item(qid) => Self::Thing(Thing::from(("Entity", Id::from(qid.0)))),
            ClaimValueData::Property(pid) => {
                Self::Thing(Thing::from(("Property", Id::from(pid.0))))
            }
            ClaimValueData::Lexeme(lid) => Self::Thing(Thing::from(("Lexeme", Id::from(lid.0)))),
            _ => Self::ClaimValueData(cvd),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    pub id: Option<Thing>,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub id: Thing,
    pub value: ClaimData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityMini {
    // Table: Entity, Property, Lexeme
    pub id: Option<Thing>,
    pub label: String,
    // Claims Table
    pub claims: Thing,
    pub description: String,
}

impl EntityMini {
    pub async fn from_entity(entity: Entity) -> (Claims, Self) {
        let thing_claim = Thing::from(("Claims", get_id_entity(&entity).await.id));

        (
            Claims {
                id: Some(thing_claim.clone()),
                ..Self::flatten_claims(entity.claims.clone()).await
            },
            Self {
                id: Some(get_id_entity(&entity).await),
                label: get_name(&entity).await,
                claims: thing_claim,
                description: get_description(&entity).await,
            },
        )
    }

    async fn flatten_claims(claims: Vec<(Pid, ClaimValue)>) -> Claims {
        Claims {
            id: None,
            claims: {
                let futures = claims.iter().map(|(pid, claim_value)| async {
                    let mut flattened = vec![Claim {
                        id: Thing::from(("Property", Id::from(pid.0))),
                        value: ClaimData::from_cvd(claim_value.data.clone()).await,
                    }];

                    let inner_futures = claim_value.qualifiers.iter().map(
                        |(qualifier_pid, qualifier_value)| async {
                            let qualifier_data = ClaimData::from_cvd(qualifier_value.clone()).await;
                            Claim {
                                id: Thing::from(("Claims", Id::from(qualifier_pid.0))),
                                value: qualifier_data,
                            }
                        },
                    );
                    flattened.extend(join_all(inner_futures).await);
                    flattened
                });

                join_all(futures).await.into_iter().flatten().collect()
            },
        }
    }
}

async fn get_id_entity(entity: &Entity) -> Thing {
    let (id, tb) = match entity.id {
        WikiId::EntityId(qid) => (qid.0, "Entity".to_string()),
        WikiId::PropertyId(pid) => (pid.0, "Property".to_string()),
        WikiId::LexemeId(lid) => (lid.0, "Lexeme".to_string()),
        _ => todo!("Not implemented"),
    };

    Thing::from((tb, Id::from(id)))
}

async fn get_name(entity: &Entity) -> String {
    entity
        .labels
        .get(&Lang(get_wikidata_lang().await.to_string()))
        .map(|label| label.to_string())
        .unwrap_or_default()
}

async fn get_description(entity: &Entity) -> String {
    entity
        .descriptions
        .get(&Lang(get_wikidata_lang().await.to_string()))
        .cloned()
        .unwrap_or_default()
}
