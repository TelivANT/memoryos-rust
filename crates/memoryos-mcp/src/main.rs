//! MemoryOS MCP Server
//!
//! Exposes MemoryOS memory management capabilities via the Model Context Protocol (MCP).
//! Supports stdio transport (for Claude Desktop) and SSE transport (for remote agents).

mod tools;

use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use std::net::SocketAddr;
use tools::MemoryOsServer;

#[derive(Debug, Clone, clap::ValueEnum)]
enum TransportMode {
    Stdio,
    Sse,
}

#[derive(Parser, Debug)]
#[command(name = "memoryos-mcp", about = "MemoryOS MCP Server")]
struct Cli {
    /// Transport mode: stdio or sse
    #[arg(long, default_value = "stdio")]
    transport: TransportMode,

    /// SSE listen address (only used with --transport sse)
    #[arg(long, default_value = "127.0.0.1:3001")]
    sse_addr: SocketAddr,

    /// MemoryOS Gateway base URL for API calls
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    gateway_url: String,

    /// API key for authenticating with the Gateway
    #[arg(long, env = "MEMORYOS_API_KEY", default_value = "")]
    api_key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("memoryos_mcp=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    tracing::info!(
        transport = ?cli.transport,
        gateway_url = %cli.gateway_url,
        "Starting MemoryOS MCP Server"
    );

    let server = MemoryOsServer::new(cli.gateway_url, cli.api_key);

    match cli.transport {
        TransportMode::Stdio => {
            let service = server
                .serve(stdio())
                .await
                .inspect_err(|e| tracing::error!("MCP server error: {e}"))?;
            service.waiting().await?;
        }
        TransportMode::Sse => {
            return Err("SSE transport is not yet implemented. Use --transport stdio.".into());
        }
    }

    Ok(())
}
