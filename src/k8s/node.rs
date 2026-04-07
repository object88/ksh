use anyhow::{Context, Result, anyhow};
use k8s_openapi::api::core::v1::Node;
use kube::{Api, api::ListParams};

use crate::k8s::client::Client;

pub async fn find_node(client: &Client) -> Result<Node> {
	let node_api: Api<Node> = Api::all(client.client());

	// Get first (arbitrary)
	let nodes = node_api
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
