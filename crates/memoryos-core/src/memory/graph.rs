use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

// 编译时验证的正则表达式（避免运行时 panic）
static NODE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-zA-Z0-9_]+)\[(.*?)\]").expect("BUG: Invalid node regex pattern"));

static EDGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z0-9_]+)\s*-+>\|(.*?)\|\s*([a-zA-Z0-9_]+)")
        .expect("BUG: Invalid edge regex pattern")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    pub id: String,    // Normalized ID (e.g., "apple_inc")
    pub label: String, // Display Name (e.g., "Apple Inc.")
    pub relations: Vec<GraphRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelation {
    pub predicate: String,    // e.g., "manufactures"
    pub target_id: String,    // e.g., "iphone"
    pub target_label: String, // e.g., "iPhone"
}

pub struct GraphManager;

impl GraphManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse Mermaid text into structured Entities
    pub fn parse_mermaid(&self, mermaid_text: &str) -> Vec<GraphEntity> {
        let mut entities = std::collections::HashMap::new();

        // 1. Extract Nodes: A[Apple]
        for caps in NODE_REGEX.captures_iter(mermaid_text) {
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            entities.entry(id.clone()).or_insert(GraphEntity {
                id,
                label,
                relations: vec![],
            });
        }

        // 2. Extract Edges: A -->|makes| B
        for caps in EDGE_REGEX.captures_iter(mermaid_text) {
            let source_id = caps[1].to_string();
            let predicate = caps[2].to_string();
            let target_id = caps[3].to_string();

            // Get target label first (before mutable borrow)
            let target_label = entities
                .get(&target_id)
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

impl Default for GraphManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mermaid_nodes() {
        let gm = GraphManager::new();
        let mermaid = "graph TD\n    A[Apple]\n    B[Banana]";
        let entities = gm.parse_mermaid(mermaid);
        assert_eq!(entities.len(), 2);
        let labels: Vec<&str> = entities.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"Apple"));
        assert!(labels.contains(&"Banana"));
    }

    #[test]
    fn test_parse_mermaid_edges() {
        let gm = GraphManager::new();
        let mermaid = "graph TD\n    A[Apple]\n    B[iPhone]\n    A -->|makes| B";
        let entities = gm.parse_mermaid(mermaid);
        let apple = entities.iter().find(|e| e.id == "A").unwrap();
        assert_eq!(apple.relations.len(), 1);
        assert_eq!(apple.relations[0].predicate, "makes");
        assert_eq!(apple.relations[0].target_id, "B");
    }

    #[test]
    fn test_parse_mermaid_empty() {
        let gm = GraphManager::new();
        let entities = gm.parse_mermaid("");
        assert!(entities.is_empty());
    }

    #[test]
    fn test_parse_mermaid_no_nodes() {
        let gm = GraphManager::new();
        let entities = gm.parse_mermaid("just some random text");
        assert!(entities.is_empty());
    }

    #[test]
    fn test_to_mermaid_roundtrip() {
        let gm = GraphManager::new();
        let entities = vec![GraphEntity {
            id: "A".to_string(),
            label: "Apple".to_string(),
            relations: vec![GraphRelation {
                predicate: "makes".to_string(),
                target_id: "B".to_string(),
                target_label: "iPhone".to_string(),
            }],
        }];
        let mermaid = gm.to_mermaid(&entities);
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("A[Apple]"));
        assert!(mermaid.contains("makes"));
    }

    #[test]
    fn test_parse_mermaid_edge_creates_missing_source() {
        let gm = GraphManager::new();
        let mermaid = "X -->|rel| Y[Target]";
        let entities = gm.parse_mermaid(mermaid);
        let x = entities.iter().find(|e| e.id == "X");
        assert!(x.is_some());
        assert_eq!(x.unwrap().label, "X");
    }

    #[test]
    fn test_to_mermaid_empty() {
        let gm = GraphManager::new();
        let mermaid = gm.to_mermaid(&[]);
        assert_eq!(mermaid, "graph TD\n");
    }

    #[test]
    fn test_default() {
        let _gm = GraphManager::default();
    }
}
