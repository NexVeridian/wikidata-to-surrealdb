use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use serde_json::Value;
use std::fs::File;
use wikidata::{ClaimValue, Entity, Lang, Pid, WikiId};

pub async fn get_entity(path: &str) -> Result<Entity, Error> {
    let mut file = File::open(path)?;
    let json: Value = from_reader(&mut file)?;
    let data = Entity::from_json(json).expect("Failed to parse JSON");
    Ok(data)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Id {
    pub entity_type: String,
    pub id: u64,
}

impl Id {
    pub fn to_string(&self) -> (String, String) {
        (self.entity_type.clone(), self.id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMini {
    // In English
    pub label: String,
    pub claims: Vec<(Pid, ClaimValue)>,
    pub description: String,
}

impl EntityMini {
    pub fn from_entity(entity: Entity) -> (Id, Self) {
        (
            get_id(&entity),
            Self {
                label: get_name(&entity),
                claims: entity.claims.clone(),
                description: get_description(&entity).unwrap_or("".to_string()),
            },
        )
    }
}

fn get_id(entity: &Entity) -> Id {
    let (id, entity_type) = match entity.id {
        WikiId::EntityId(qid) => (qid.0, "Entity".to_string()),
        WikiId::PropertyId(pid) => (pid.0, "Property".to_string()),
        WikiId::LexemeId(lid) => (lid.0, "Lexeme".to_string()),
        _ => todo!("Not implemented"),
    };

    Id { id, entity_type }
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
