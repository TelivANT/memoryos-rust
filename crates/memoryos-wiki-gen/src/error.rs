use thiserror::Error;

#[derive(Error, Debug)]
pub enum WikiGenError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Parse error in {file}: {message}")]
    Parse { file: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type WikiGenResult<T> = Result<T, WikiGenError>;
