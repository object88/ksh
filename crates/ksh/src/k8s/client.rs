use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result};
use hyper_util::rt::TokioExecutor;
use kube::{
	Client as K8sClient, Config,
	client::ConfigExt,
	config::{KubeConfigOptions, Kubeconfig},
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
	pub fn with_cluster(mut self, cluster: impl Into<Cluster>) -> Self {
		self.cluster = Some(cluster.into());
		self
	}
	pub fn with_context(mut self, context: impl Into<Context>) -> Self {
		self.context = Some(context.into());
		self
	}

	pub fn with_kubeconfig(mut self, kubeconfig: PathBuf) -> Self {
		self.context_path = Some(kubeconfig);
		self
	}

	pub fn with_namespace(mut self, namespace: impl Into<Namespace>) -> Self {
		self.namespace = Some(namespace.into());
		self
	}

	pub async fn build(mut self) -> Result<Client> {
		let mut opt = KubeConfigOptions {
			..Default::default()
		};
		if let Some(ctr) = self.cluster.take() {
			opt.cluster = Some(ctr.0)
		}
		if let Some(ctx) = self.context.take() {
			opt.context = Some(ctx.0)
		}
		let cfg = if let Some(path) = self.context_path {
			let f = Kubeconfig::read_from(path)?;
			Config::from_custom_kubeconfig(f, &opt).await?
		} else {
			Config::from_kubeconfig(&opt).await?
		};

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
}
