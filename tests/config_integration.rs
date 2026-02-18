//! Config 热更新集成测试

use memoryos_core::ConfigManager;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_config_hot_reload_mtime() {
    // 创建临时配置文件
    let temp_config = "/tmp/test_config_mtime.toml";
    let initial_config = r#"
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4
timeout_seconds = 60

[llm]
provider = "openai"
api_key = "sk-test"
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6333"

[router]
enable = true
"#;

    fs::write(temp_config, initial_config).unwrap();
    std::env::set_var("MEMORYOS_CONFIG", temp_config);

    // 创建 ConfigManager
    let mut manager = ConfigManager::new().unwrap();
    let config1 = manager.get();
    assert_eq!(config1.server.port, 8080);

    // 等待确保 mtime 不同
    sleep(Duration::from_millis(100)).await;

    // 修改配置
    let updated_config = initial_config.replace("port = 8080", "port = 9090");
    fs::write(temp_config, updated_config).unwrap();

    // 触发热更新
    let changed = manager.reload_if_changed().unwrap();
    assert!(changed);

    let config2 = manager.get();
    assert_eq!(config2.server.port, 9090);

    // 清理
    fs::remove_file(temp_config).ok();
    std::env::remove_var("MEMORYOS_CONFIG");
}

#[tokio::test]
async fn test_config_hot_reload_content_hash() {
    // 测试 K8s ConfigMap 模式（content-hash）
    let temp_config = "/tmp/test_config_hash.toml";
    let initial_config = r#"
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4
timeout_seconds = 60

[llm]
provider = "openai"
api_key = "sk-test"
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6333"

[router]
enable = true
"#;

    fs::write(temp_config, initial_config).unwrap();
    std::env::set_var("MEMORYOS_CONFIG", temp_config);
    std::env::set_var("MEMORYOS_CONFIG_HASH_CHECK", "true");

    let mut manager = ConfigManager::new().unwrap();
    
    // 修改配置（不改 mtime）
    let updated_config = initial_config.replace("port = 8080", "port = 7070");
    fs::write(temp_config, updated_config).unwrap();

    // 触发热更新（content-hash 模式）
    let changed = manager.reload_if_changed().unwrap();
    assert!(changed);

    let config = manager.get();
    assert_eq!(config.server.port, 7070);

    // 清理
    fs::remove_file(temp_config).ok();
    std::env::remove_var("MEMORYOS_CONFIG");
    std::env::remove_var("MEMORYOS_CONFIG_HASH_CHECK");
}

#[test]
fn test_config_validation_errors() {
    let temp_config = "/tmp/test_config_invalid.toml";
    let invalid_config = r#"
[server]
host = "0.0.0.0"
port = 0
worker_threads = 4
timeout_seconds = 60

[llm]
provider = "openai"
api_key = "sk-test"
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6333"

[router]
enable = true
"#;

    fs::write(temp_config, invalid_config).unwrap();
    std::env::set_var("MEMORYOS_CONFIG", temp_config);

    let result = ConfigManager::new();
    assert!(result.is_err());

    // 清理
    fs::remove_file(temp_config).ok();
    std::env::remove_var("MEMORYOS_CONFIG");
}
