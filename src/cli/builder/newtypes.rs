use clap::{
	Arg, Command,
	builder::{TypedValueParser, ValueParserFactory},
	error::ErrorKind,
};

/// Newtype for k8s::client::Cluster, so that it can implement clap builder's
/// value parser
#[derive(Clone, Debug)]
pub struct Cluster(crate::k8s::client::Cluster);

impl From<Cluster> for crate::k8s::client::Cluster {
	fn from(value: Cluster) -> crate::k8s::client::Cluster {
		value.0
	}
}

impl ValueParserFactory for Cluster {
	type Parser = ClusterValueParser;
	fn value_parser() -> Self::Parser {
		ClusterValueParser
	}
}

#[derive(Clone, Debug)]
pub struct ClusterValueParser;

impl TypedValueParser for ClusterValueParser {
	type Value = Cluster;

	fn parse_ref(
		&self,
		cmd: &Command,
		_arg: Option<&Arg>,
		value: &std::ffi::OsStr,
	) -> Result<Self::Value, clap::Error> {
		let x = match value.to_str() {
			Some(x) => x,
			None => return Err(clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd)),
		};
		Ok(Cluster(x.to_string().into()))
	}
}

/// Newtype for k8s::client::Context, so that it can implement clap builder's
/// value parser
#[derive(Clone, Debug)]
pub struct Context(crate::k8s::client::Context);

impl From<Context> for crate::k8s::client::Context {
	fn from(value: Context) -> crate::k8s::client::Context {
		value.0
	}
}

impl ValueParserFactory for Context {
	type Parser = ContextValueParser;
	fn value_parser() -> Self::Parser {
		ContextValueParser
	}
}

#[derive(Clone, Debug)]
pub struct ContextValueParser;

impl TypedValueParser for ContextValueParser {
	type Value = Context;

	fn parse_ref(
		&self,
		cmd: &Command,
		_arg: Option<&Arg>,
		value: &std::ffi::OsStr,
	) -> Result<Self::Value, clap::Error> {
		let x = match value.to_str() {
			Some(x) => x,
			None => return Err(clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd)),
		};
		Ok(Context(x.to_string().into()))
	}
}

/// Newtype for k8s::client::Namespace, so that it can implement clap builder's
/// value parser
#[derive(Clone, Debug)]
pub struct Namespace(crate::k8s::client::Namespace);

impl From<Namespace> for crate::k8s::client::Namespace {
	fn from(value: Namespace) -> crate::k8s::client::Namespace {
		value.0
	}
}

impl ValueParserFactory for Namespace {
	type Parser = NamespaceValueParser;
	fn value_parser() -> Self::Parser {
		NamespaceValueParser
	}
}

#[derive(Clone, Debug)]
pub struct NamespaceValueParser;

impl TypedValueParser for NamespaceValueParser {
	type Value = Namespace;

	fn parse_ref(
		&self,
		cmd: &Command,
		_arg: Option<&Arg>,
		value: &std::ffi::OsStr,
	) -> Result<Self::Value, clap::Error> {
		let x = match value.to_str() {
			Some(x) => x,
			None => return Err(clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd)),
		};
		Ok(Namespace(x.to_string().into()))
	}
}
