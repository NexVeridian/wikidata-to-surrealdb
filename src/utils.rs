use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use serde_json::Value;
use std::fs::File;
use surrealdb::sql::Thing;
use wikidata::ClaimValueData;
use wikidata::{ClaimValue, Entity, Lang, Pid, WikiId};

pub async fn get_entity(path: &str) -> Result<Entity, Error> {
    // From here - https://www.wikidata.org/wiki/Special:EntityData/P1476.json
    let mut file = File::open(path)?;
    let json: Value = from_reader(&mut file)?;
    let data = Entity::from_json(json).expect("Failed to parse JSON");
    Ok(data)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMini {
    // In English
    pub label: String,
    pub claims: Vec<(Thing, ClaimValueData)>,
    pub description: String,
}

impl EntityMini {
    pub fn from_entity(entity: Entity) -> (Thing, Self) {
        (
            get_id(&entity),
            Self {
                label: get_name(&entity),
                claims: Self::flatten_claims(entity.claims.clone()),
                description: get_description(&entity).unwrap_or("".to_string()),
            },
        )
    }

    fn flatten_claims(claims: Vec<(Pid, ClaimValue)>) -> Vec<(Thing, ClaimValueData)> {
        claims
            .iter()
            .flat_map(|(pid, claim_value)| {
                let mut flattened = vec![(
                    Thing {
                        id: pid.0.into(),
                        tb: "Property".to_string(),
                    },
                    claim_value.data.clone(),
                )];

                flattened.extend(claim_value.qualifiers.iter().map(
                    |(qualifier_pid, qualifier_value)| {
                        (
                            Thing {
                                id: qualifier_pid.0.into(),
                                tb: "Property".to_string(),
                            },
                            qualifier_value.clone(),
                        )
                    },
                ));
                flattened
            })
            .collect()
    }
}

fn get_id(entity: &Entity) -> Thing {
    let (id, tb) = match entity.id {
        WikiId::EntityId(qid) => (qid.0, "Entity".to_string()),
        WikiId::PropertyId(pid) => (pid.0, "Property".to_string()),
        WikiId::LexemeId(lid) => (lid.0, "Lexeme".to_string()),
        _ => todo!("Not implemented"),
    };

    Thing { id: id.into(), tb }
}

fn get_name(entity: &Entity) -> String {
    entity
        .labels
        .get(&Lang("en".to_string()))
        .expect("No label found")
        .to_string()
}

fn get_description(entity: &Entity) -> Option<String> {
    entity.descriptions.get(&Lang("en".to_string())).cloned()
}
