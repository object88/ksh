use std::collections::BTreeMap;

use anyhow::{Context, Result};
use k8s_openapi::{
	api::core::v1::{PersistentVolumeClaim, PersistentVolumeClaimSpec, VolumeResourceRequirements},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{
	Api,
	api::{ObjectMeta, PostParams},
};

use crate::k8s::client::Client;

pub struct Manager {
	api: Api<PersistentVolumeClaim>,
}

pub fn new(client: &Client) -> Manager {
	let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.client(), client.namespace());
	Manager { api: pvc_api }
}

impl Manager {
	pub fn generate(&self) -> Result<PersistentVolumeClaim> {
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

		Ok(pvc_spec)
	}

	pub async fn instantiate(
		&self,
		pvc_spec: &PersistentVolumeClaim,
	) -> Result<PersistentVolumeClaim> {
		let pvc = self
			.api
			.create(&PostParams::default(), pvc_spec)
			.await
			.context("failed to create pvc")?;

		Ok(pvc)
	}
}
