use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::attribute::AttributeValue;

/// Entity type with hierarchical classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Person,
    Location,
    Item,
    Organization,
    Concept,
    Resource,
    Ability,
    Event(String),
    Custom { category: String, label: String },
}

impl EntityType {
    pub fn category(&self) -> &str {
        match self {
            Self::Person => "Person",
            Self::Location => "Location",
            Self::Item => "Item",
            Self::Organization => "Organization",
            Self::Concept => "Concept",
            Self::Resource => "Resource",
            Self::Ability => "Ability",
            Self::Event(_) => "Event",
            Self::Custom { category, .. } => category,
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Self::Person => "Person".into(),
            Self::Location => "Location".into(),
            Self::Item => "Item".into(),
            Self::Organization => "Organization".into(),
            Self::Concept => "Concept".into(),
            Self::Resource => "Resource".into(),
            Self::Ability => "Ability".into(),
            Self::Event(s) => format!("Event:{}", s),
            Self::Custom { category, label } => format!("{}:{}", category, label),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Person" => Self::Person,
            "Location" => Self::Location,
            "Item" => Self::Item,
            "Organization" => Self::Organization,
            "Concept" => Self::Concept,
            "Resource" => Self::Resource,
            "Ability" => Self::Ability,
            other => {
                if let Some(rest) = other.strip_prefix("Event:") {
                    Self::Event(rest.to_string())
                } else if let Some((cat, label)) = other.split_once(':') {
                    Self::Custom {
                        category: cat.to_string(),
                        label: label.to_string(),
                    }
                } else {
                    Self::Custom {
                        category: "Other".into(),
                        label: other.to_string(),
                    }
                }
            }
        }
    }
}

/// Knowledge graph entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub entity_type: EntityType,
    pub attributes: HashMap<String, AttributeValue>,
    pub description: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
    pub source: Option<String>,
    pub tags: Vec<String>,
}
