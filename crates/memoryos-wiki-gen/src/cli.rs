use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::info;

use memoryos_wiki_gen::config::WikiGenConfig;
use memoryos_wiki_gen::WikiGenerator;

#[derive(Parser)]
#[command(name = "memoryos-wiki-gen")]
#[command(about = "Generate wiki documentation from code repositories")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Generate {
        #[arg(long, default_value = ".")]
        repo: PathBuf,

        #[arg(long, default_value = "wiki-output")]
        output: PathBuf,

        #[arg(long)]
        config: Option<PathBuf>,

        #[arg(long, default_value = "false")]
        incremental: bool,
    },

    Parse {
        #[arg(long, default_value = ".")]
        repo: PathBuf,

        #[arg(long)]
        output_ir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            repo,
            output,
            config,
            incremental: _,
        } => {
            let mut wiki_config = if let Some(config_path) = config {
                WikiGenConfig::from_file(&config_path)?
            } else {
                WikiGenConfig::default()
            };

            wiki_config.output.output_dir = output.to_string_lossy().to_string();

            let generator = WikiGenerator::new(wiki_config);
            let repo_path = std::fs::canonicalize(&repo)?;

            info!("Generating wiki for {}", repo_path.display());
            generator.generate(&repo_path).await?;
            info!("Done!");
        }

        Commands::Parse { repo, output_ir } => {
            let wiki_config = WikiGenConfig::default();
            let generator = WikiGenerator::new(wiki_config);
            let repo_path = std::fs::canonicalize(&repo)?;

            info!("Parsing {}", repo_path.display());
            let ir = generator.parse_only(&repo_path)?;

            if let Some(output_path) = output_ir {
                let json = serde_json::to_string_pretty(&ir)?;
                std::fs::write(&output_path, json)?;
                info!("IR written to {}", output_path.display());
            } else {
                println!(
                    "Files: {}, Symbols: {}, References: {}, Endpoints: {}",
                    ir.files.len(),
                    ir.symbols.len(),
                    ir.references.len(),
                    ir.endpoints.len(),
                );
            }
        }
    }

    Ok(())
}
