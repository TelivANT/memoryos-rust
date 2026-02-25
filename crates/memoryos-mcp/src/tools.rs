//! MCP Tool definitions for MemoryOS
//!
//! Each tool maps to a MemoryOS Gateway API endpoint.
//! The MCP server acts as a thin proxy, forwarding requests to the Gateway.

use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use std::future::Future;
use std::sync::Arc;

/// HTTP client for calling the MemoryOS Gateway API.
#[derive(Debug, Clone)]
struct GatewayClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GatewayClient {
    fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<String, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Gateway request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {e}"))?;
        if !status.is_success() {
            return Err(format!("Gateway returned {status}: {text}"));
        }
        Ok(text)
    }

    async fn get(&self, path: &str) -> Result<String, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Gateway request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {e}"))?;
        if !status.is_success() {
            return Err(format!("Gateway returned {status}: {text}"));
        }
        Ok(text)
    }
}

// ── Tool Input Types ──────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMemoryInput {
    #[schemars(description = "User ID to store memory for")]
    pub user_id: String,
    #[schemars(description = "Message content to store")]
    pub content: String,
    #[schemars(description = "Role of the message sender (user or assistant)")]
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "user".to_string()
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchMemoriesInput {
    #[schemars(description = "User ID to search memories for")]
    pub user_id: String,
    #[schemars(description = "Search query text")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    5
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMemoriesInput {
    #[schemars(description = "User ID to retrieve memories for")]
    pub user_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteMemoryInput {
    #[schemars(description = "User ID whose data should be deleted (GDPR)")]
    pub user_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QueryGraphInput {
    #[schemars(description = "User ID for graph query")]
    pub user_id: String,
    #[schemars(description = "Entity or relationship to query")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatInput {
    #[schemars(description = "User ID for the conversation")]
    pub user_id: String,
    #[schemars(description = "User message to send")]
    pub message: String,
    #[schemars(description = "LLM model to use (e.g. gpt-4, claude-3)")]
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HealthInput {}

// ── MCP Server ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MemoryOsServer {
    gateway: Arc<GatewayClient>,
    tool_router: ToolRouter<Self>,
}

impl MemoryOsServer {
    pub fn new(gateway_url: String, api_key: String) -> Self {
        Self {
            gateway: Arc::new(GatewayClient::new(gateway_url, api_key)),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl MemoryOsServer {
    #[tool(description = "Store a message into short-term memory (STM) for a user")]
    async fn add_memory(&self, Parameters(input): Parameters<AddMemoryInput>) -> String {
        let body = serde_json::json!({
            "user_id": input.user_id,
            "messages": [{
                "role": input.role,
                "content": input.content
            }]
        });
        match self.gateway.post("/v1/memory/add", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Search user memories by semantic similarity")]
    async fn search_memories(&self, Parameters(input): Parameters<SearchMemoriesInput>) -> String {
        let body = serde_json::json!({
            "user_id": input.user_id,
            "query": input.query,
            "limit": input.limit
        });
        match self.gateway.post("/v1/memory/retrieve", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get all memories for a user (short-term, mid-term, long-term)")]
    async fn get_memories(&self, Parameters(input): Parameters<GetMemoriesInput>) -> String {
        let body = serde_json::json!({
            "user_id": input.user_id
        });
        match self.gateway.post("/v1/memory/retrieve", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Delete all user data (GDPR compliance)")]
    async fn delete_memory(&self, Parameters(input): Parameters<DeleteMemoryInput>) -> String {
        let body = serde_json::json!({
            "user_id": input.user_id,
            "action": "delete_all"
        });
        match self.gateway.post("/v1/security/gdpr/delete", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Query the knowledge graph for entity relationships")]
    async fn query_graph(&self, Parameters(input): Parameters<QueryGraphInput>) -> String {
        let body = serde_json::json!({
            "user_id": input.user_id,
            "query": input.query
        });
        match self.gateway.post("/v1/graph/query", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Send a chat message with memory-augmented context")]
    async fn chat(&self, Parameters(input): Parameters<ChatInput>) -> String {
        let mut body = serde_json::json!({
            "user_id": input.user_id,
            "messages": [{
                "role": "user",
                "content": input.message
            }]
        });
        if let Some(model) = input.model {
            body["model"] = serde_json::Value::String(model);
        }
        match self.gateway.post("/v1/chat", &body).await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Check MemoryOS system health status")]
    async fn health_check(&self, Parameters(_input): Parameters<HealthInput>) -> String {
        match self.gateway.get("/v1/health").await {
            Ok(resp) => resp,
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for MemoryOsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            instructions: Some(
                "MemoryOS MCP Server - AI Agent memory management. \
                 Store, search, and retrieve memories across short-term, mid-term, \
                 and long-term tiers. Supports knowledge graphs and GDPR compliance."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "memoryos-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}
