use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::env;
use surrealdb::sql::{Id, Thing};
use wikidata::{ClaimValue, ClaimValueData, Entity, Lang, Pid, WikiId};

lazy_static! {
    static ref WIKIDATA_LANG: String = env::var("WIKIDATA_LANG")
        .expect("WIKIDATA_LANG not set")
        .to_string();
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClaimData {
    Thing(Thing),
    ClaimValueData(ClaimValueData),
}

impl ClaimData {
    fn from_cvd(cvd: ClaimValueData) -> Self {
        match cvd {
            ClaimValueData::Item(qid) => ClaimData::Thing(Thing::from(("Entity", Id::from(qid.0)))),
            ClaimValueData::Property(pid) => {
                ClaimData::Thing(Thing::from(("Property", Id::from(pid.0))))
            }
            ClaimValueData::Lexeme(lid) => {
                ClaimData::Thing(Thing::from(("Lexeme", Id::from(lid.0))))
            }
            _ => ClaimData::ClaimValueData(cvd),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMini {
    // Table: Entity, Property, Lexeme
    pub id: Option<Thing>,
    pub label: String,
    // Claims Table
    pub claims: Thing,
    pub description: String,
}

impl EntityMini {
    pub fn from_entity(entity: Entity) -> (Claims, Self) {
        let thing_claim = Thing::from(("Claims", get_id_entity(&entity).id));

        (
            Claims {
                id: Some(thing_claim.clone()),
                ..Self::flatten_claims(entity.claims.clone())
            },
            Self {
                id: Some(get_id_entity(&entity)),
                label: get_name(&entity),
                claims: thing_claim,
                description: get_description(&entity),
            },
        )
    }

    fn flatten_claims(claims: Vec<(Pid, ClaimValue)>) -> Claims {
        Claims {
            id: None,
            claims: claims
                .iter()
                .flat_map(|(pid, claim_value)| {
                    let mut flattened = vec![Claim {
                        id: Thing::from(("Property", Id::from(pid.0))),
                        value: ClaimData::from_cvd(claim_value.data.clone()),
                    }];

                    flattened.extend(claim_value.qualifiers.iter().map(
                        |(qualifier_pid, qualifier_value)| Claim {
                            id: Thing::from(("Claims", Id::from(qualifier_pid.0))),
                            value: ClaimData::from_cvd(qualifier_value.clone()),
                        },
                    ));
                    flattened
                })
                .collect(),
        }
    }
}

fn get_id_entity(entity: &Entity) -> Thing {
    let (id, tb) = match entity.id {
        WikiId::EntityId(qid) => (qid.0, "Entity".to_string()),
        WikiId::PropertyId(pid) => (pid.0, "Property".to_string()),
        WikiId::LexemeId(lid) => (lid.0, "Lexeme".to_string()),
        _ => todo!("Not implemented"),
    };

    Thing::from((tb, Id::from(id)))
}

fn get_name(entity: &Entity) -> String {
    entity
        .labels
        .get(&Lang(WIKIDATA_LANG.to_string()))
        .map(|label| label.to_string())
        .unwrap_or_default()
}

fn get_description(entity: &Entity) -> String {
    entity
        .descriptions
        .get(&Lang(WIKIDATA_LANG.to_string()))
        .cloned()
        .unwrap_or_default()
}
