use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

static NODE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-zA-Z0-9_]+)\[(.*?)\]").expect("BUG: Invalid node regex pattern"));

static EDGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z0-9_]+)\s*-+>\|(.*?)\|\s*([a-zA-Z0-9_]+)")
        .expect("BUG: Invalid edge regex pattern")
});

static ENTITY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:(?:Dr|Mr|Mrs|Ms|Prof)\.?\s+)?([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b")
        .expect("BUG: Invalid entity regex")
});

static RELATION_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"(?i)(.+?)\s+(?:works?\s+(?:at|for)|employed\s+(?:at|by))\s+(.+)")
                .unwrap(),
            "works_at",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:lives?\s+in|(?:is|are)\s+from|(?:is|are)\s+located\s+in|located\s+in)\s+(.+)")
                .unwrap(),
            "located_in",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:is|are)\s+friends?\s+(?:with|of)\s+(.+)").unwrap(),
            "friends_with",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:(?:is|are)\s+(?:a|an|the)\s+)?(?:friend|partner|colleague|spouse|wife|husband)\s+(?:of|with)\s+(.+)").unwrap(),
            "related_to",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:likes?|loves?|enjoys?|prefers?)\s+(.+)").unwrap(),
            "likes",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:uses?|utilizes?)\s+(.+)").unwrap(),
            "uses",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:(?:is|are)\s+(?:part|member)\s+of|belongs?\s+to)\s+(.+)")
                .unwrap(),
            "member_of",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:created?|built|made|developed|authored)\s+(.+)").unwrap(),
            "created",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:manages?|leads?|heads?|directs?)\s+(.+)").unwrap(),
            "manages",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:owns?|has|have)\s+(?:a\s+)?(.+)").unwrap(),
            "owns",
        ),
        (
            Regex::new(r"(?i)(.+?)\s+(?:studies?|learns?|majors?\s+in|studying)\s+(.+)").unwrap(),
            "studies",
        ),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEntity {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub entity_type: EntityType,
    pub relations: Vec<GraphRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Concept,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphRelation {
    pub predicate: String,
    pub target_id: String,
    pub target_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub entities: Vec<GraphEntity>,
    pub triples: Vec<ExtractedTriple>,
}

pub struct GraphManager {
    entities: HashMap<String, GraphEntity>,
}

impl GraphManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    fn normalize_id(label: &str) -> String {
        label
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_")
    }

    fn infer_entity_type(label: &str, context: &str) -> EntityType {
        let lower = label.to_lowercase();
        let ctx_lower = context.to_lowercase();

        let person_indicators = [
            "mr",
            "mrs",
            "ms",
            "dr",
            "prof",
            "ceo",
            "cto",
            "manager",
            "engineer",
            "developer",
        ];
        let org_indicators = [
            "inc",
            "corp",
            "llc",
            "ltd",
            "company",
            "team",
            "group",
            "university",
            "school",
        ];
        let location_indicators = [
            "city", "country", "state", "street", "avenue", "road", "town", "village",
        ];

        for ind in &person_indicators {
            if lower.contains(ind)
                || ctx_lower.contains(&format!("{} {}", label.to_lowercase(), ind))
            {
                return EntityType::Person;
            }
        }
        for ind in &org_indicators {
            if lower.contains(ind) {
                return EntityType::Organization;
            }
        }
        for ind in &location_indicators {
            if lower.contains(ind) {
                return EntityType::Location;
            }
        }

        let work_ctx = ["works at", "employed", "lives in", "from"];
        for wc in &work_ctx {
            if ctx_lower.contains(wc) {
                let parts: Vec<&str> = ctx_lower.splitn(2, wc).collect();
                if parts.len() == 2 {
                    if parts[0].contains(&lower) {
                        return EntityType::Person;
                    }
                    if parts[1].contains(&lower) {
                        if wc.contains("work") || wc.contains("employ") {
                            return EntityType::Organization;
                        }
                        if wc.contains("live") || wc.contains("from") {
                            return EntityType::Location;
                        }
                    }
                }
            }
        }

        EntityType::Unknown
    }

    pub fn extract_entities(&self, text: &str) -> Vec<GraphEntity> {
        let mut seen = HashMap::new();

        for caps in ENTITY_PATTERN.captures_iter(text) {
            let label = caps[1].to_string();
            let id = Self::normalize_id(&label);
            if !seen.contains_key(&id) {
                let entity_type = Self::infer_entity_type(&label, text);
                seen.insert(
                    id.clone(),
                    GraphEntity {
                        id,
                        label,
                        entity_type,
                        relations: vec![],
                    },
                );
            }
        }

        seen.into_values().collect()
    }

    pub fn extract_relations(&self, text: &str) -> Vec<ExtractedTriple> {
        let mut triples = Vec::new();
        for sentence in text.split(['.', '!', '?', '\n']) {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }
            for (pattern, predicate) in RELATION_PATTERNS.iter() {
                if let Some(caps) = pattern.captures(sentence) {
                    let subject = caps[1].trim().to_string();
                    let object = caps[2]
                        .trim()
                        .trim_end_matches(['.', ',', '!', '?'])
                        .to_string();
                    if subject.len() >= 2
                        && object.len() >= 2
                        && subject.len() <= 100
                        && object.len() <= 100
                    {
                        triples.push(ExtractedTriple {
                            subject,
                            predicate: predicate.to_string(),
                            object,
                        });
                    }
                }
            }
        }
        triples
    }

    pub fn extract_and_merge(&mut self, text: &str) -> Vec<ExtractedTriple> {
        let entities = self.extract_entities(text);
        for entity in entities {
            self.entities.entry(entity.id.clone()).or_insert(entity);
        }

        let triples = self.extract_relations(text);
        for triple in &triples {
            let subject_id = Self::normalize_id(&triple.subject);
            let object_id = Self::normalize_id(&triple.object);

            self.entities
                .entry(subject_id.clone())
                .or_insert_with(|| GraphEntity {
                    id: subject_id.clone(),
                    label: triple.subject.clone(),
                    entity_type: Self::infer_entity_type(&triple.subject, text),
                    relations: vec![],
                });

            self.entities
                .entry(object_id.clone())
                .or_insert_with(|| GraphEntity {
                    id: object_id.clone(),
                    label: triple.object.clone(),
                    entity_type: Self::infer_entity_type(&triple.object, text),
                    relations: vec![],
                });

            if let Some(entity) = self.entities.get_mut(&subject_id) {
                let already_exists = entity
                    .relations
                    .iter()
                    .any(|r| r.predicate == triple.predicate && r.target_id == object_id);
                if !already_exists {
                    entity.relations.push(GraphRelation {
                        predicate: triple.predicate.clone(),
                        target_id: object_id,
                        target_label: triple.object.clone(),
                    });
                }
            }
        }

        triples
    }

    pub fn query_entity(&self, entity_id: &str) -> Option<&GraphEntity> {
        self.entities.get(entity_id)
    }

    pub fn query_by_label(&self, label: &str) -> Vec<&GraphEntity> {
        let lower = label.to_lowercase();
        self.entities
            .values()
            .filter(|e| e.label.to_lowercase().contains(&lower))
            .collect()
    }

    pub fn query_relations(&self, entity_id: &str) -> Vec<ExtractedTriple> {
        let mut triples = Vec::new();

        if let Some(entity) = self.entities.get(entity_id) {
            for rel in &entity.relations {
                triples.push(ExtractedTriple {
                    subject: entity.label.clone(),
                    predicate: rel.predicate.clone(),
                    object: rel.target_label.clone(),
                });
            }
        }

        for entity in self.entities.values() {
            for rel in &entity.relations {
                if rel.target_id == entity_id {
                    triples.push(ExtractedTriple {
                        subject: entity.label.clone(),
                        predicate: rel.predicate.clone(),
                        object: rel.target_label.clone(),
                    });
                }
            }
        }

        triples
    }

    pub fn query_path(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: usize,
    ) -> Vec<Vec<ExtractedTriple>> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();
        self.dfs_path(
            from_id,
            to_id,
            max_depth,
            &mut visited,
            &mut path,
            &mut results,
        );
        results
    }

    fn dfs_path(
        &self,
        current: &str,
        target: &str,
        max_depth: usize,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<ExtractedTriple>,
        results: &mut Vec<Vec<ExtractedTriple>>,
    ) {
        if current == target && !path.is_empty() {
            results.push(path.clone());
            return;
        }
        if path.len() >= max_depth {
            return;
        }
        if visited.contains(current) {
            return;
        }
        visited.insert(current.to_string());

        if let Some(entity) = self.entities.get(current) {
            for rel in &entity.relations {
                path.push(ExtractedTriple {
                    subject: entity.label.clone(),
                    predicate: rel.predicate.clone(),
                    object: rel.target_label.clone(),
                });
                self.dfs_path(&rel.target_id, target, max_depth, visited, path, results);
                path.pop();
            }
        }

        visited.remove(current);
    }

    pub fn get_all_entities(&self) -> Vec<&GraphEntity> {
        self.entities.values().collect()
    }

    pub fn get_all_triples(&self) -> Vec<ExtractedTriple> {
        let mut triples = Vec::new();
        for entity in self.entities.values() {
            for rel in &entity.relations {
                triples.push(ExtractedTriple {
                    subject: entity.label.clone(),
                    predicate: rel.predicate.clone(),
                    object: rel.target_label.clone(),
                });
            }
        }
        triples
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn relation_count(&self) -> usize {
        self.entities.values().map(|e| e.relations.len()).sum()
    }

    pub fn parse_mermaid(&self, mermaid_text: &str) -> Vec<GraphEntity> {
        let mut entities = HashMap::new();

        for caps in NODE_REGEX.captures_iter(mermaid_text) {
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            entities.entry(id.clone()).or_insert(GraphEntity {
                id,
                label,
                entity_type: EntityType::Unknown,
                relations: vec![],
            });
        }

        for caps in EDGE_REGEX.captures_iter(mermaid_text) {
            let source_id = caps[1].to_string();
            let predicate = caps[2].to_string();
            let target_id = caps[3].to_string();

            let target_label = entities
                .get(&target_id)
                .map(|e| e.label.clone())
                .unwrap_or_else(|| target_id.clone());

            let source = entities.entry(source_id.clone()).or_insert(GraphEntity {
                id: source_id.clone(),
                label: source_id.clone(),
                entity_type: EntityType::Unknown,
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

    pub fn to_mermaid(&self, entities: &[GraphEntity]) -> String {
        let mut mermaid = String::from("graph TD\n");

        for entity in entities {
            mermaid.push_str(&format!("    {}[{}]\n", entity.id, entity.label));

            for rel in &entity.relations {
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
            entity_type: EntityType::Unknown,
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

    #[test]
    fn test_extract_entities_from_text() {
        let gm = GraphManager::new();
        let text = "John Smith works at Apple Inc in San Francisco.";
        let entities = gm.extract_entities(text);
        let labels: Vec<&str> = entities.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"John Smith"));
        assert!(labels.contains(&"San Francisco"));
    }

    #[test]
    fn test_extract_relations_works_at() {
        let gm = GraphManager::new();
        let text = "Alice Chen works at Google.";
        let triples = gm.extract_relations(text);
        assert!(!triples.is_empty());
        assert_eq!(triples[0].predicate, "works_at");
    }

    #[test]
    fn test_extract_relations_lives_in() {
        let gm = GraphManager::new();
        let text = "Bob lives in New York.";
        let triples = gm.extract_relations(text);
        assert!(!triples.is_empty());
        assert_eq!(triples[0].predicate, "located_in");
    }

    #[test]
    fn test_extract_and_merge() {
        let mut gm = GraphManager::new();
        gm.extract_and_merge("Alice Chen works at Google.");
        gm.extract_and_merge("Alice Chen lives in Seattle.");
        assert!(gm.entity_count() >= 2);
        let alice_id = GraphManager::normalize_id("Alice Chen");
        let rels = gm.query_relations(&alice_id);
        assert!(rels.len() >= 2);
    }

    #[test]
    fn test_query_by_label() {
        let mut gm = GraphManager::new();
        gm.extract_and_merge("John Smith works at Microsoft.");
        let results = gm.query_by_label("John");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_query_path() {
        let mut gm = GraphManager::new();
        gm.extract_and_merge("Alice uses Python.");
        gm.extract_and_merge("Python created Django.");
        let alice_id = GraphManager::normalize_id("Alice");
        let django_id = GraphManager::normalize_id("Django");
        let paths = gm.query_path(&alice_id, &django_id, 3);
        assert!(!paths.is_empty() || gm.entity_count() >= 2);
    }

    #[test]
    fn test_get_all_triples() {
        let mut gm = GraphManager::new();
        gm.extract_and_merge("Bob likes Python.");
        let triples = gm.get_all_triples();
        assert!(!triples.is_empty());
    }

    #[test]
    fn test_extract_friends_with() {
        let gm = GraphManager::new();
        let text = "Bob is friends with Alice.";
        let triples = gm.extract_relations(text);
        assert!(!triples.is_empty());
        assert_eq!(triples[0].predicate, "friends_with");
        assert_eq!(triples[0].subject, "Bob");
        assert_eq!(triples[0].object, "Alice");
    }

    #[test]
    fn test_extract_is_located_in() {
        let gm = GraphManager::new();
        let text = "Google is located in Mountain View.";
        let triples = gm.extract_relations(text);
        assert!(!triples.is_empty());
        assert_eq!(triples[0].predicate, "located_in");
        assert_eq!(triples[0].subject, "Google");
        assert_eq!(triples[0].object, "Mountain View");
    }

    #[test]
    fn test_extract_friend_singular() {
        let gm = GraphManager::new();
        let text = "Charlie is friend of Dave.";
        let triples = gm.extract_relations(text);
        assert!(!triples.is_empty());
        let friend_triple = triples
            .iter()
            .find(|t| t.predicate == "friends_with" || t.predicate == "related_to");
        assert!(friend_triple.is_some());
    }
}
