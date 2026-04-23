use std::path::PathBuf;

use ksh::k8s::client::Builder;

use crate::{
	builder::newtypes::{Cluster, Context, Namespace},
	constants::{
		FLAG_COMMON_CLUSTER, FLAG_COMMON_CONTEXT, FLAG_COMMON_KUBECONFIG_FILE, FLAG_COMMON_NAMESPACE,
	},
};

pub trait BuilderExt {
	fn with_common_flags(self, sub: &clap::ArgMatches) -> Self;
}

impl BuilderExt for Builder {
	fn with_common_flags(mut self, sub: &clap::ArgMatches) -> Self {
		if let Some(x) = sub.get_one::<Cluster>(FLAG_COMMON_CLUSTER) {
			self = self.with_cluster(x.clone());
		}
		if let Some(x) = sub.get_one::<Context>(FLAG_COMMON_CONTEXT) {
			self = self.with_context(x.clone());
		}
		if let Some(x) = sub.get_one::<PathBuf>(FLAG_COMMON_KUBECONFIG_FILE) {
			self = self.with_kubeconfig(x.clone());
		}
		if let Some(x) = sub.get_one::<Namespace>(FLAG_COMMON_NAMESPACE) {
			self = self.with_namespace(x.clone());
		}
		self
	}
}
