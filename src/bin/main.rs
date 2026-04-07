use anyhow::Result;
use ksh::cli;

#[tokio::main]
async fn main() -> Result<()> {
	let root = cli::cli::Cli::new();
	root.parse().await
}
