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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(contents: Vec<MultiModalContent>) -> MultiModalMessage {
        MultiModalMessage {
            role: "user".to_string(),
            contents,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_extract_text_from_text_content() {
        let msg = make_message(vec![MultiModalContent::Text {
            content: "hello world".to_string(),
        }]);
        assert_eq!(msg.extract_text(), "hello world");
    }

    #[test]
    fn test_extract_text_from_image_caption() {
        let msg = make_message(vec![MultiModalContent::Image {
            url: "https://example.com/img.png".to_string(),
            caption: Some("a cat".to_string()),
            embedding: None,
        }]);
        assert_eq!(msg.extract_text(), "a cat");
    }

    #[test]
    fn test_extract_text_from_audio_transcript() {
        let msg = make_message(vec![MultiModalContent::Audio {
            url: "https://example.com/audio.mp3".to_string(),
            transcript: Some("hello from audio".to_string()),
            embedding: None,
        }]);
        assert_eq!(msg.extract_text(), "hello from audio");
    }

    #[test]
    fn test_extract_text_from_video_transcript() {
        let msg = make_message(vec![MultiModalContent::Video {
            url: "https://example.com/video.mp4".to_string(),
            transcript: Some("video speech".to_string()),
            thumbnail: None,
        }]);
        assert_eq!(msg.extract_text(), "video speech");
    }

    #[test]
    fn test_extract_text_mixed_contents() {
        let msg = make_message(vec![
            MultiModalContent::Text {
                content: "first".to_string(),
            },
            MultiModalContent::Image {
                url: "https://example.com/img.png".to_string(),
                caption: Some("second".to_string()),
                embedding: None,
            },
            MultiModalContent::Audio {
                url: "https://example.com/a.mp3".to_string(),
                transcript: Some("third".to_string()),
                embedding: None,
            },
        ]);
        assert_eq!(msg.extract_text(), "first second third");
    }

    #[test]
    fn test_extract_text_no_text_content() {
        let msg = make_message(vec![MultiModalContent::Image {
            url: "https://example.com/img.png".to_string(),
            caption: None,
            embedding: None,
        }]);
        assert_eq!(msg.extract_text(), "");
    }

    #[test]
    fn test_get_embeddings_from_image() {
        let msg = make_message(vec![MultiModalContent::Image {
            url: "https://example.com/img.png".to_string(),
            caption: None,
            embedding: Some(vec![1.0, 2.0, 3.0]),
        }]);
        let embeddings = msg.get_embeddings();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0], vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_get_embeddings_from_audio() {
        let msg = make_message(vec![MultiModalContent::Audio {
            url: "https://example.com/a.mp3".to_string(),
            transcript: None,
            embedding: Some(vec![4.0, 5.0]),
        }]);
        let embeddings = msg.get_embeddings();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0], vec![4.0, 5.0]);
    }

    #[test]
    fn test_get_embeddings_skips_text_and_video() {
        let msg = make_message(vec![
            MultiModalContent::Text {
                content: "text".to_string(),
            },
            MultiModalContent::Video {
                url: "https://example.com/v.mp4".to_string(),
                transcript: None,
                thumbnail: None,
            },
        ]);
        assert!(msg.get_embeddings().is_empty());
    }

    #[test]
    fn test_get_embeddings_none_embedding() {
        let msg = make_message(vec![MultiModalContent::Image {
            url: "https://example.com/img.png".to_string(),
            caption: None,
            embedding: None,
        }]);
        assert!(msg.get_embeddings().is_empty());
    }

    #[test]
    fn test_multimodal_content_serialization() {
        let content = MultiModalContent::Text {
            content: "test".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        let deserialized: MultiModalContent = serde_json::from_str(&json).unwrap();
        assert_eq!(content, deserialized);
    }

    #[test]
    fn test_empty_message() {
        let msg = make_message(vec![]);
        assert_eq!(msg.extract_text(), "");
        assert!(msg.get_embeddings().is_empty());
    }
}
