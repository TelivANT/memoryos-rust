#![cfg(test)]

use super::{GitConnector, LocalConnector, StorageConnector};
use tempfile::TempDir;

#[tokio::test]
async fn test_local_connector() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    tokio::fs::write(&test_file, b"hello world").await.unwrap();

    let mut connector = LocalConnector::new(temp_dir.path().to_path_buf());
    connector.connect().await.unwrap();

    // Test exists
    assert!(connector.exists("test.txt").await.unwrap());
    assert!(!connector.exists("nonexistent.txt").await.unwrap());

    // Test read
    let content = connector.read_file("test.txt").await.unwrap();
    assert_eq!(content, b"hello world");

    // Test metadata
    let meta = connector.metadata("test.txt").await.unwrap();
    assert_eq!(meta.size, 11);
    assert!(!meta.is_dir);
}

#[tokio::test]
#[ignore] // Requires network
async fn test_git_connector_public_repo() {
    // Test with a small public repo (no auth needed)
    let mut connector = GitConnector::new("https://github.com/rust-lang/rustlings.git".to_string())
        .with_branch("main".to_string());

    // This will clone the repo
    connector.connect().await.unwrap();

    // Test exists
    assert!(connector.exists("README.md").await.unwrap());

    // Test read
    let content = connector.read_file("README.md").await.unwrap();
    assert!(!content.is_empty());
}
