use dotenv_codegen::dotenv;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use wikidata::{ClaimValue, ClaimValueData, Entity, Lang, Pid, WikiId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    pub claims: Vec<(Thing, ClaimValueData)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMini {
    pub label: String,
    pub claims: Thing,
    pub description: String,
}

impl EntityMini {
    pub fn from_entity(entity: Entity) -> (Thing, (Thing, Claims), Self) {
        let thing_claim = Thing {
            id: get_id(&entity).id,
            tb: "Claims".to_string(),
        };

        (
            get_id(&entity),
            (
                thing_claim.clone(),
                Self::flatten_claims(entity.claims.clone()),
            ),
            Self {
                label: get_name(&entity),
                claims: thing_claim,
                description: get_description(&entity).unwrap_or("".to_string()),
            },
        )
    }

    fn flatten_claims(claims: Vec<(Pid, ClaimValue)>) -> Claims {
        Claims {
            claims: claims
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
                .collect(),
        }
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
        .get(&Lang(dotenv!("WIKIDATA_LANG").to_string()))
        .expect("No label found")
        .to_string()
}

fn get_description(entity: &Entity) -> Option<String> {
    entity
        .descriptions
        .get(&Lang(dotenv!("WIKIDATA_LANG").to_string()))
        .cloned()
}
