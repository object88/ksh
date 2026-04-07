use std::collections::BTreeMap;

use anyhow::{Context, Result};
use k8s_openapi::{
	api::core::v1::{
		HostPathVolumeSource, NodeSelector, NodeSelectorRequirement, NodeSelectorTerm,
		PersistentVolume, PersistentVolumeSpec, VolumeNodeAffinity,
	},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{
	Api, Resource,
	api::{ObjectMeta, PostParams},
};

use crate::k8s::client::Client;

pub fn generate_pv(node_name: String) -> PersistentVolume {
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

	pv_spec
}

pub async fn instantiate_pv(
	client: &Client,
	pv_spec: &PersistentVolume,
) -> Result<PersistentVolume> {
	let pv_api: Api<PersistentVolume> = Api::all(client.client());
	pv_api
		.create(&PostParams::default(), &pv_spec)
		.await
		.with_context(|| {
			let name = (|| -> Option<String> { Some(pv_spec.meta().name.as_ref()?.clone()) })()
				.unwrap_or("unknown".to_string());
			let path = (|| -> Option<String> {
				let spec = pv_spec.spec.as_ref()?;
				let na = spec.node_affinity.as_ref()?;
				let r = na.required.as_ref()?;
				let nst = r.node_selector_terms.first()?;
				let nsr = nst.match_expressions.as_ref()?.first()?;
				let val = nsr.values.as_ref()?.first()?;
				Some(val.clone())
			})()
			.unwrap_or("unknown".to_string());
			format!(
				"failed to create persistent volume with local volume {} name {}",
				path, name
			)
		})
}
