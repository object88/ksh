use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::terminal::size;
use futures::{SinkExt, StreamExt};
use kube::{
	Config,
	api::{AttachedProcess, TerminalSize},
	config::KubeConfigOptions,
};
use tracing::warn;

#[derive(Default)]
pub struct Builder {
	context_path: Option<PathBuf>,
}

impl Builder {
	pub async fn build(self) -> Result<Client> {
		// let c = K8sClient::new(service, default_namespace)
		let opt = KubeConfigOptions {
			..Default::default()
		};
		let cfg = Config::from_kubeconfig(&opt).await?;

		// let config = Config::infer().await?;
		// let service = ServiceBuilder::new()
		// 	.layer(cfg.base_uri_layer())
		// 	.option_layer(cfg.auth_layer().context("cfg.auth_layer")?)
		// 	.map_err(BoxError::from)
		// 	.service(hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build_http());
		// let client = kube::Client::new(service, cfg.default_namespace);

		let client = kube::Client::try_from(cfg).context("trying to build client from config")?;

		let info = client
			.apiserver_version()
			.await
			.context("getting apiserver version")?;
		warn!("info: {:?}", info);

		Ok(Client { client })
	}
}

pub struct Client {
	client: kube::Client,
}

impl Client {
	pub fn builder() -> Builder {
		Builder::default()
	}

	pub fn client(&self) -> kube::Client {
		self.client.clone()
	}

	pub fn namespace(&self) -> &str {
		self.client.default_namespace()
	}

	async fn handle_resize(&self, attached_pod: &mut AttachedProcess) -> Result<()> {
		let (cols, rows) = size()?;

		let mut resize_writer = attached_pod.terminal_size().unwrap();
		resize_writer
			.send(TerminalSize {
				width: cols,
				height: rows,
			})
			.await?;

		tokio::spawn(async move {
			let mut events = crossterm::event::EventStream::new();
			while let Some(Ok(crossterm::event::Event::Resize(cols, rows))) = events.next().await {
				let _ = resize_writer
					.send(TerminalSize {
						width: cols,
						height: rows,
					})
					.await;
			}
		});

		Ok(())
	}

	async fn handle_streams(&self, attached_pod: &mut AttachedProcess) -> Result<()> {
		let mut stdin_writer = attached_pod.stdin().unwrap();
		let mut stdout_reader = attached_pod.stdout().unwrap();

		let mut stdin = tokio::io::stdin();
		let mut stdout = tokio::io::stdout();

		tokio::spawn(async move {
			tokio::io::copy(&mut stdin, &mut stdin_writer)
				.await
				.unwrap();
		});
		tokio::spawn(async move {
			tokio::io::copy(&mut stdout_reader, &mut stdout)
				.await
				.unwrap();
		});
		Ok(())
	}
}
