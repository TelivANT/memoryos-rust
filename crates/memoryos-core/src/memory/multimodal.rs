//! Multi-modal memory support

use serde::{Deserialize, Serialize};

/// Multi-modal content type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultiModalContent {
    Text {
        content: String,
    },
    Image {
        url: String,
        caption: Option<String>,
        embedding: Option<Vec<f32>>,
    },
    Audio {
        url: String,
        transcript: Option<String>,
        embedding: Option<Vec<f32>>,
    },
    Video {
        url: String,
        transcript: Option<String>,
        thumbnail: Option<String>,
    },
}

/// Multi-modal message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModalMessage {
    pub role: String,
    pub contents: Vec<MultiModalContent>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MultiModalMessage {
    /// Extract text content for search
    pub fn extract_text(&self) -> String {
        self.contents
            .iter()
            .filter_map(|c| match c {
                MultiModalContent::Text { content } => Some(content.clone()),
                MultiModalContent::Image { caption, .. } => caption.clone(),
                MultiModalContent::Audio { transcript, .. } => transcript.clone(),
                MultiModalContent::Video { transcript, .. } => transcript.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get all embeddings
    pub fn get_embeddings(&self) -> Vec<Vec<f32>> {
        self.contents
            .iter()
            .filter_map(|c| match c {
                MultiModalContent::Image { embedding, .. } => embedding.clone(),
                MultiModalContent::Audio { embedding, .. } => embedding.clone(),
                _ => None,
            })
            .collect()
    }
}
