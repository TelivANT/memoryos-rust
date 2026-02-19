use crate::error::Result;

/// Incremental memory summarizer
pub struct IncrementalSummarizer {
    current_summary: String,
    message_count: usize,
    threshold: usize,
}

impl IncrementalSummarizer {
    pub fn new(threshold: usize) -> Self {
        Self {
            current_summary: String::new(),
            message_count: 0,
            threshold,
        }
    }

    pub async fn add_messages<F, Fut>(
        &mut self,
        messages: Vec<String>,
        summarize_fn: F,
    ) -> Result<Option<String>>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        // Append new messages
        for msg in messages {
            self.current_summary.push_str(&msg);
            self.current_summary.push('\n');
            self.message_count += 1;
        }

        // Check if threshold reached
        if self.message_count >= self.threshold {
            let summary = summarize_fn(self.current_summary.clone()).await?;
            self.current_summary = summary.clone();
            self.message_count = 0;
            return Ok(Some(summary));
        }

        Ok(None)
    }

    pub fn current_summary(&self) -> &str {
        &self.current_summary
    }

    pub fn message_count(&self) -> usize {
        self.message_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_incremental_summarizer() {
        let mut summarizer = IncrementalSummarizer::new(3);

        let result = summarizer
            .add_messages(
                vec!["msg1".to_string(), "msg2".to_string()],
                |text| async move { Ok(format!("Summary: {}", text)) },
            )
            .await;

        assert!(result.unwrap().is_none());
        assert_eq!(summarizer.message_count(), 2);

        let result = summarizer
            .add_messages(vec!["msg3".to_string()], |text| async move {
                Ok(format!("Summary: {}", text))
            })
            .await;

        assert!(result.unwrap().is_some());
        assert_eq!(summarizer.message_count(), 0);
    }
}
