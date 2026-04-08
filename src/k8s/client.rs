use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use futures::{SinkExt, StreamExt};
use kube::{
	Config,
	api::{AttachedProcess, TerminalSize},
	config::KubeConfigOptions,
};
use scopeguard;
use tracing::{info, warn};

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

impl Default for Builder {
	fn default() -> Self {
		Self {
			context_path: Default::default(),
		}
	}
}

pub struct Client {
	client: kube::Client,
}

impl Client {
	pub fn builder() -> Builder {
		Builder::default()
	}

	pub async fn run(&self) -> Result<()> {
		let node_api = crate::k8s::node::new(self);

		enable_raw_mode()?;
		let _guard = scopeguard::guard((), |_| {
			// Ensure that we drop out of raw mode.
			let _ = disable_raw_mode();
		});

		let node = match node_api.find_node().await {
			Ok(x) => x,
			Err(e) => anyhow::bail!("failed to find a node: {}", e),
		};
		let node_name = node.metadata.name.context("failed to get node")?;

		warn!("about to create pv");
		let pv = crate::k8s::pv::generate_pv(node_name);
		crate::k8s::pv::instantiate_pv(self, &pv).await?;

		warn!("about to create pvc");
		let pvc = crate::k8s::pvc::generate_pvc()?;
		crate::k8s::pvc::instantiate_pvc(self, &pvc).await?;

		warn!("about to create pod");
		let pod = crate::k8s::pod::generate_pod();
		let pod = crate::k8s::pod::instantiate_pod(self, &pod).await?;
		let pod_name = pod.metadata.name.as_ref().unwrap().clone();
		warn!("runrunrun");
		let mut attached_process = crate::k8s::pod::runrunrun(self, pod_name).await?;

		warn!("attaching resize and streams");
		self.handle_resize(&mut attached_process).await?;
		self.handle_streams(&mut attached_process).await?;

		let status = attached_process.take_status().unwrap().await;
		info!("{:?}", status);

		Ok(())
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
