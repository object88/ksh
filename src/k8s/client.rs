use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result};
use crossterm::terminal::size;
use futures::{SinkExt, StreamExt};
use hyper_util::rt::TokioExecutor;
use kube::{
	Client as K8sClient, Config,
	api::{AttachedProcess, TerminalSize},
	client::ConfigExt,
	config::KubeConfigOptions,
};
use tower::{BoxError, ServiceBuilder};
use tracing::warn;

#[derive(Clone, Debug)]
pub struct Cluster(String);

impl From<String> for Cluster {
	fn from(value: String) -> Self {
		Cluster(value)
	}
}

#[derive(Clone, Debug)]
pub struct Context(String);

impl From<String> for Context {
	fn from(value: String) -> Self {
		Context(value)
	}
}

#[derive(Clone, Debug)]
pub struct Namespace(String);

impl From<String> for Namespace {
	fn from(value: String) -> Self {
		Namespace(value)
	}
}

#[derive(Default)]
pub struct Builder {
	context_path: Option<PathBuf>,
	cluster: Option<Cluster>,
	context: Option<Context>,
	namespace: Option<Namespace>,
}

impl Builder {
	pub fn with_cluster(mut self, cluster: Cluster) -> Self {
		self.cluster = Some(cluster);
		self
	}
	pub fn with_context(mut self, context: Context) -> Self {
		self.context = Some(context);
		self
	}

	pub fn with_namespace(mut self, namespace: Namespace) -> Self {
		self.namespace = Some(namespace);
		self
	}

	pub async fn build(mut self) -> Result<Client> {
		// let c = K8sClient::new(service, default_namespace)
		let mut opt = KubeConfigOptions {
			..Default::default()
		};
		if let Some(ctr) = self.cluster.take() {
			opt.cluster = Some(ctr.0)
		}
		if let Some(ctx) = self.context.take() {
			opt.context = Some(ctx.0)
		}
		let cfg = Config::from_kubeconfig(&opt).await?;

		let ns = self
			.namespace
			.take()
			.unwrap_or_else(|| Namespace(cfg.default_namespace.clone()))
			.0;

		let https = cfg.rustls_https_connector()?;
		let service = ServiceBuilder::new()
			.layer(cfg.base_uri_layer())
			.option_layer(cfg.auth_layer()?)
			.map_err(BoxError::from)
			.service(hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https));
		let client = K8sClient::new(service, ns);

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
