use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{
	Container, PersistentVolumeClaimVolumeSource, Pod, PodSpec, Volume, VolumeMount,
};
use kube::{
	Api, ResourceExt,
	api::{AttachParams, AttachedProcess, ObjectMeta, PostParams, WatchEvent, WatchParams},
};
use tracing::{info, warn};

use crate::k8s::client::Client;

pub struct Manager {
	api: Api<Pod>,
}

pub fn new(client: &Client) -> Manager {
	let api: Api<Pod> = Api::namespaced(client.client(), client.namespace());
	Manager { api }
}

impl Manager {
	// TODO: bind pod to particular node
	pub fn generate(&self) -> Pod {
		let pod_spec = Pod {
			metadata: ObjectMeta {
				name: Some("foo".to_string()),
				..Default::default()
			},
			spec: Some(PodSpec {
				containers: vec![Container {
					command: Some(vec![
						"/bin/sh".to_string(),
						"-c".to_string(),
						"sleep infinity".to_string(),
					]),
					image: Some("rust:1-bullseye".to_string()),
					name: "main".to_string(),
					volume_mounts: Some(vec![VolumeMount {
						mount_path: "/mnt/foo".to_string(),
						name: "foo".to_string(),
						..Default::default()
					}]),
					..Default::default()
				}],
				volumes: Some(vec![Volume {
					name: "foo".to_string(),
					persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
						claim_name: "foo".to_string(),
						read_only: Some(false),
					}),
					..Default::default()
				}]),
				..Default::default()
			}),
			..Default::default()
		};

		pod_spec
	}

	pub async fn instantiate(&self, pod_spec: &Pod) -> Result<Pod> {
		let pod = self
			.api
			.create(&PostParams::default(), &pod_spec)
			.await
			.context("creating pod")?;
		let pod_name = pod.metadata.name.as_ref().unwrap();
		warn!("created pod {}", pod_name);

		Ok(pod)
	}

	pub async fn runrunrun(&self, pod_name: String) -> Result<AttachedProcess> {
		// Wait for the pod to be ready
		let watch_params =
			WatchParams::default().fields(format!("metadata.name={}", pod_name).as_str());
		let mut stream = self.api.watch(&watch_params, "0").await?.boxed();
		while let Some(status) = stream.try_next().await? {
			match status {
				WatchEvent::Added(o) => {
					info!("Added {}", o.name_any());
				},
				WatchEvent::Modified(o) => {
					let s = o.status.as_ref().expect("missing status");
					if s.phase.clone().unwrap_or_default() == "Running" {
						info!("Ready to attach to {}", o.name_any());
						break;
					}
				},
				_ => {},
			}
		}

		let command = vec!["bash"];
		let attach_params = AttachParams::interactive_tty();
		let attached_process = self
			.api
			.exec(pod_name.as_str(), command, &attach_params)
			.await
			.context("attaching to process")?;

		Ok(attached_process)
	}
}
