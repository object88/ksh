use anyhow::Result;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
	let root = Cli::new();
	root.parse().await
}
