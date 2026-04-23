use anyhow::{Context, Result, anyhow};
use k8s_openapi::api::core::v1::Node;
use kube::{Api, api::ListParams};

use crate::k8s::client::Client;

pub struct NodeClient {
	api: Api<Node>,
}

pub fn new(client: &Client) -> NodeClient {
	let node_api: Api<Node> = Api::all(client.client());
	NodeClient { api: node_api }
}

impl NodeClient {
	pub async fn find_node(&self) -> Result<Node> {
		// Get first (arbitrary)
		let nodes = self
			.api
			.list(&ListParams {
				..Default::default()
			})
			.await
			.context("node_api list")
			.unwrap();
		let node = match nodes.into_iter().next() {
			Some(node) => node,
			None => {
				return Err(anyhow!("no nodes"));
			},
		};

		Ok(node)
	}
}
