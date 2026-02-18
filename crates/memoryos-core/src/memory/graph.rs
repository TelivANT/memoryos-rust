use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    pub id: String, // Normalized ID (e.g., "apple_inc")
    pub label: String, // Display Name (e.g., "Apple Inc.")
    pub relations: Vec<GraphRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelation {
    pub predicate: String, // e.g., "manufactures"
    pub target_id: String, // e.g., "iphone"
    pub target_label: String, // e.g., "iPhone"
}

pub struct GraphManager {
    node_regex: Regex,
    edge_regex: Regex,
}

impl GraphManager {
    pub fn new() -> Self {
        Self { 
            // Matches: A[Label]
            node_regex: Regex::new(r"([a-zA-Z0-9_]+)\[(.*?)\]").unwrap(),
            // Matches: A -->|Predicate| B
            edge_regex: Regex::new(r"([a-zA-Z0-9_]+)\s*-+>\|(.*?)\|\s*([a-zA-Z0-9_]+)").unwrap(),
        }
    }

    /// Parse Mermaid text into structured Entities
    pub fn parse_mermaid(&self, mermaid_text: &str) -> Vec<GraphEntity> {
        let mut entities = std::collections::HashMap::new();

        // 1. Extract Nodes: A[Apple]
        for caps in self.node_regex.captures_iter(mermaid_text) {
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            entities.entry(id.clone()).or_insert(GraphEntity {
                id,
                label,
                relations: vec![],
            });
        }

        // 2. Extract Edges: A -->|makes| B
        for caps in self.edge_regex.captures_iter(mermaid_text) {
            let source_id = caps[1].to_string();
            let predicate = caps[2].to_string();
            let target_id = caps[3].to_string();

            // Get target label first (before mutable borrow)
            let target_label = entities.get(&target_id)
                .map(|e| e.label.clone())
                .unwrap_or_else(|| target_id.clone());

            // Ensure source exists (create partial if missing)
            let source = entities.entry(source_id.clone()).or_insert(GraphEntity {
                id: source_id.clone(),
                label: source_id.clone(), // Fallback label
                relations: vec![],
            });

            source.relations.push(GraphRelation {
                predicate,
                target_id,
                target_label,
            });
        }

        entities.into_values().collect()
    }

    /// Convert structured Entities back to Mermaid
    pub fn to_mermaid(&self, entities: &[GraphEntity]) -> String {
        let mut mermaid = String::from("graph TD\n");
        
        for entity in entities {
            // Node definition: A[Label]
            mermaid.push_str(&format!("    {}[{}]\n", entity.id, entity.label));
            
            for rel in &entity.relations {
                // Edge definition: A -->|Rel| B
                mermaid.push_str(&format!(
                    "    {} -->|{}| {}({})\n", 
                    entity.id, rel.predicate, rel.target_id, rel.target_label
                ));
            }
        }
        mermaid
    }
}

