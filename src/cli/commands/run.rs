use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use futures::{SinkExt, StreamExt};
use kube::api::{AttachedProcess, TerminalSize};
use scopeguard;
use tracing::{info, warn};

use crate::k8s::pod::Name;

pub struct Command {
	node_api: crate::k8s::node::NodeClient,
	pv_api: crate::k8s::pv::PvClient,
	pvc_mgr: crate::k8s::pvc::Manager,
	pod_mgr: crate::k8s::pod::Manager,

	name: Name,
}

impl Command {
	pub fn new(client: crate::k8s::client::Client, name: Name) -> Command {
		let node_api = crate::k8s::node::new(&client);
		let pv_api = crate::k8s::pv::new(&client);
		let pvc_mgr = crate::k8s::pvc::new(&client);
		let pod_mgr = crate::k8s::pod::new(&client);

		Command {
			node_api,
			pv_api,
			pvc_mgr,
			pod_mgr,
			name,
		}
	}

	pub async fn run(&self) -> Result<()> {
		enable_raw_mode()?;
		let _guard = scopeguard::guard((), |_| {
			// Ensure that we drop out of raw mode.
			let _ = disable_raw_mode();
		});

		let node = match self.node_api.find_node().await {
			Ok(x) => x,
			Err(e) => anyhow::bail!("failed to find a node: {}", e),
		};
		let node_name = node.metadata.name.context("failed to get node")?;

		warn!("about to create pv");
		let pv = self.pv_api.generate(node_name);
		self.pv_api.instantiate(&pv).await?;

		warn!("about to create pvc");
		let pvc = self.pvc_mgr.generate()?;
		self.pvc_mgr.instantiate(&pvc).await?;

		warn!("about to create pod");
		let pod = self.pod_mgr.generate(&self.name);
		let pod = self.pod_mgr.instantiate(&pod).await?;
		let pod_name = pod.metadata.name.as_ref().unwrap().clone();
		warn!("runrunrun");
		let mut attached_process = self.pod_mgr.runrunrun(pod_name).await?;

		warn!("attaching resize and streams");
		self.handle_resize(&mut attached_process).await?;
		self.handle_streams(&mut attached_process).await?;

		let status = attached_process.take_status().unwrap().await;
		info!("{:?}", status);

		Ok(())
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
