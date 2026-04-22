use clap::{Arg, Command, builder::PathBufValueParser};

use crate::cli::{
	builder::newtypes::{Cluster, Context, Namespace},
	constants::{
		FLAG_COMMON_CLUSTER, FLAG_COMMON_CONTEXT, FLAG_COMMON_KUBECONFIG_FILE, FLAG_COMMON_NAMESPACE,
	},
};

pub trait CommandExt {
	fn foo(self) -> Self;
}

impl CommandExt for Command {
	fn foo(self) -> Self {
		self
			.arg(
				Arg::new(FLAG_COMMON_KUBECONFIG_FILE)
					.long(FLAG_COMMON_KUBECONFIG_FILE)
					.value_parser(PathBufValueParser::new()),
			)
			.arg(
				Arg::new(FLAG_COMMON_CLUSTER)
					.long(FLAG_COMMON_CLUSTER)
					.value_parser(clap::value_parser!(Cluster)),
			)
			.arg(
				Arg::new(FLAG_COMMON_CONTEXT)
					.long(FLAG_COMMON_CONTEXT)
					.value_parser(clap::value_parser!(Context)),
			)
			.arg(
				Arg::new(FLAG_COMMON_NAMESPACE)
					.long(FLAG_COMMON_NAMESPACE)
					.value_parser(clap::value_parser!(Namespace)),
			)
	}
}
