use ksh::k8s::client::Builder;

use crate::{
	builder::newtypes::{Cluster, Context, Namespace},
	constants::{FLAG_COMMON_CLUSTER, FLAG_COMMON_CONTEXT, FLAG_COMMON_NAMESPACE},
};

pub trait BuilderExt {
	fn foo(self, sub: &clap::ArgMatches) -> Self;
}

impl BuilderExt for Builder {
	fn foo(mut self, sub: &clap::ArgMatches) -> Self {
		if let Some(x) = sub.get_one::<Cluster>(FLAG_COMMON_CLUSTER) {
			self = self.with_cluster(x.clone());
		}
		if let Some(x) = sub.get_one::<Context>(FLAG_COMMON_CONTEXT) {
			self = self.with_context(x.clone());
		}
		if let Some(x) = sub.get_one::<Namespace>(FLAG_COMMON_NAMESPACE) {
			self = self.with_namespace(x.clone());
		}
		self
	}
}
