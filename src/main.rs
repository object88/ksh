use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use futures::{SinkExt, StreamExt, TryStreamExt};
use k8s_openapi::{
	api::core::v1::{
		Container, HostPathVolumeSource, Node, NodeSelector, NodeSelectorRequirement, NodeSelectorTerm,
		PersistentVolume, PersistentVolumeClaim, PersistentVolumeClaimSpec,
		PersistentVolumeClaimVolumeSource, PersistentVolumeSpec, Pod, PodSpec, Volume, VolumeMount,
		VolumeNodeAffinity, VolumeResourceRequirements,
	},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{
	Api, Client, ResourceExt,
	api::{AttachParams, ListParams, ObjectMeta, PostParams, TerminalSize, WatchEvent, WatchParams},
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
	let client = Client::try_default().await?;

	let namespace = client.default_namespace();

	let node_api: Api<Node> = Api::all(client.clone());
	let pod_api: Api<Pod> = Api::namespaced(client.clone(), namespace);
	let pv_api: Api<PersistentVolume> = Api::all(client.clone());
	let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace);

	// Get first (arbitrary)
	let nodes = node_api
		.list(&ListParams {
			..Default::default()
		})
		.await
		.unwrap();
	let node = match nodes.into_iter().next() {
		Some(node) => node,
		None => {
			return Err(anyhow!("no nodes"));
		},
	};
	let node_name = node.metadata.name.unwrap();

	let pv_spec = PersistentVolume {
		metadata: ObjectMeta {
			name: Some("foo".to_string()),
			..Default::default()
		},
		spec: Some(PersistentVolumeSpec {
			access_modes: Some(vec!["ReadWriteOnce".to_string()]),
			capacity: Some({
				let mut capacity = BTreeMap::new();
				capacity.insert("storage".to_string(), Quantity("1Gi".to_string()));
				capacity
			}),
			host_path: Some(HostPathVolumeSource {
				path: "/Users/object88/code/object88/packs".to_string(),
				type_: Some("DirectoryOrCreate".to_string()),
			}),
			node_affinity: Some(VolumeNodeAffinity {
				required: Some(NodeSelector {
					node_selector_terms: vec![NodeSelectorTerm {
						match_expressions: Some(vec![NodeSelectorRequirement {
							key: "kubernetes.io/hostname".to_string(),
							operator: "In".to_string(),
							values: Some(vec![node_name]),
						}]),
						..Default::default()
					}],
					..Default::default()
				}),
				..Default::default()
			}),
			persistent_volume_reclaim_policy: Some("Retain".to_string()),
			storage_class_name: Some("local-path".to_string()),
			..Default::default()
		}),
		..Default::default()
	};

	let _pv = pv_api.create(&PostParams::default(), &pv_spec).await?;

	let pvc_spec = PersistentVolumeClaim {
		metadata: ObjectMeta {
			name: Some("foo".to_string()),
			..Default::default()
		},
		spec: Some(PersistentVolumeClaimSpec {
			access_modes: Some(vec!["ReadWriteOnce".to_string()]),
			resources: Some(VolumeResourceRequirements {
				requests: Some({
					let mut capacity = BTreeMap::new();
					capacity.insert("storage".to_string(), Quantity("1Gi".to_string()));
					capacity
				}),
				..Default::default()
			}),
			storage_class_name: Some("local-path".to_string()),
			..Default::default()
		}),
		..Default::default()
	};

	let _pvc = pvc_api.create(&PostParams::default(), &pvc_spec).await?;

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

	let pod = pod_api.create(&PostParams::default(), &pod_spec).await?;

	let pod_name = pod.metadata.name.unwrap();

	// Wait for the pod to be ready
	let watch_params = WatchParams::default().fields(format!("metadata.name={}", pod_name).as_str());
	let mut stream = pod_api.watch(&watch_params, "0").await?.boxed();
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

	enable_raw_mode()?;
	let (cols, rows) = size()?;

	let command = vec!["bash"];
	let attach_params = AttachParams::interactive_tty();
	let mut attached_pod = pod_api
		.exec(pod_name.as_str(), command, &attach_params)
		.await?;

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

	let status = attached_pod.take_status().unwrap().await;
	info!("{:?}", status);

	// On exit, always:
	disable_raw_mode()?;

	Ok(())
}
