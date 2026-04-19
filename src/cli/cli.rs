use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use anyhow::Result;
use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::error::ErrorKind;
use clap::{
	Arg, Command,
	builder::{EnumValueParser, PathBufValueParser},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::k8s::pod::Name;
use crate::{
	cli::logging::{self, ConfigLevelFilter},
	k8s::client::Client,
};

const CMD_DEBUG: &str = "debug";
const CMD_EXEC: &str = "exec";
const CMD_RUN: &str = "run";
const CMD_VERSION: &str = "version";

// These flags are used on subcommands that interact with a k8s pod, such as
// `run`
const FLAG_COMMON_CLUSTER: &str = "cluster";
const FLAG_COMMON_CONTEXT: &str = "context";
const FLAG_COMMON_KUBECONFIG_FILE: &str = "kubeconfig";
const FLAG_COMMON_NAMESPACE: &str = "namespace";

// These flags are used on all levels of the app
const FLAG_GLOBAL_LOG_LEVEL: &str = "log-level";
const FLAG_GLOBAL_PROFILES_FILE: &str = "profiles";

const CMD_RUN_FLAG_NAME: &str = "name";
const CMD_RUN_FLAG_GENERATIVE_NAME: &str = "generative-name";

pub trait Runnable {
	fn run(&self, cancel: CancellationToken) -> impl std::future::Future<Output = Result<()>> + Send;
}

enum Cmd {
	Debug,
	Exec,
	Run,
	Version,
}

impl Display for Cmd {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match *self {
			Cmd::Debug => write!(f, "{}", CMD_DEBUG),
			Cmd::Exec => write!(f, "{}", CMD_EXEC),
			Cmd::Run => write!(f, "{}", CMD_RUN),
			Cmd::Version => write!(f, "{}", CMD_VERSION),
		}
	}
}

impl From<Cmd> for clap::builder::Str {
	fn from(val: Cmd) -> Self {
		match val {
			Cmd::Debug => CMD_DEBUG.into(),
			Cmd::Exec => CMD_EXEC.into(),
			Cmd::Run => CMD_RUN.into(),
			Cmd::Version => CMD_VERSION.into(),
		}
	}
}

/// Newtype for k8s::client::Cluster, so that it can implement clap builder's
/// value parser
#[derive(Clone, Debug)]
struct Cluster(crate::k8s::client::Cluster);

impl ValueParserFactory for Cluster {
	type Parser = ClusterValueParser;
	fn value_parser() -> Self::Parser {
		ClusterValueParser
	}
}

#[derive(Clone, Debug)]
struct ClusterValueParser;

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
struct Context(crate::k8s::client::Context);

impl ValueParserFactory for Context {
	type Parser = ContextValueParser;
	fn value_parser() -> Self::Parser {
		ContextValueParser
	}
}

#[derive(Clone, Debug)]
struct ContextValueParser;

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
struct Namespace(crate::k8s::client::Namespace);

impl ValueParserFactory for Namespace {
	type Parser = NamespaceValueParser;
	fn value_parser() -> Self::Parser {
		NamespaceValueParser
	}
}

#[derive(Clone, Debug)]
struct NamespaceValueParser;

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

pub struct Cli {
	root: Command,
}

impl Cli {
	pub fn new() -> Self {
		const CRATE_NAME: &str = env!("CARGO_CRATE_NAME");

		// Get the config directory
		let config_dir = match dirs::config_dir() {
			Some(config_dir) => config_dir.join(CRATE_NAME),
			None => PathBuf::from(""),
		};

		let root = Command::new(CRATE_NAME)
			.arg(
				Arg::new(FLAG_GLOBAL_LOG_LEVEL)
					.long(FLAG_GLOBAL_LOG_LEVEL)
					.default_value(logging::DEFAULT)
					.global(true)
					.help("Set the log level")
					.long_help("Set the log level. 'trace' is the most verbose and 'off' the least verbose")
					.value_parser(EnumValueParser::<ConfigLevelFilter>::new()),
			)
			.arg(
				Arg::new(FLAG_GLOBAL_PROFILES_FILE)
					.global(true)
					.long(FLAG_GLOBAL_PROFILES_FILE)
					.default_value(config_dir.into_os_string())
					.value_parser(PathBufValueParser::new()),
			)
			.arg_required_else_help(true)
			.subcommand(Command::new(Cmd::Debug))
			.subcommand(Command::new(Cmd::Exec))
			.subcommand(
				Command::new(Cmd::Run)
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
					.arg(
						Arg::new(CMD_RUN_FLAG_GENERATIVE_NAME)
							.long(CMD_RUN_FLAG_GENERATIVE_NAME)
							.default_value("ksh-")
							.conflicts_with(CMD_RUN_FLAG_NAME),
					)
					.arg(Arg::new(CMD_RUN_FLAG_NAME).long(CMD_RUN_FLAG_NAME)),
			)
			.subcommand(Command::new(Cmd::Version));

		Cli { root }
	}

	pub async fn parse(self) -> Result<()> {
		let matches = self.root.get_matches();

		// Configure logging first; let's figure out how to report back to the world.
		let log_level: &ConfigLevelFilter = matches
			.get_one::<ConfigLevelFilter>(FLAG_GLOBAL_LOG_LEVEL)
			.unwrap_or(&logging::DEFAULT);
		logging::init(log_level);

		match matches.subcommand() {
			Some((CMD_DEBUG, _sub)) => Ok(()),
			Some((CMD_EXEC, _sub)) => Ok(()),
			Some((CMD_RUN, sub)) => {
				warn!("building builder");
				let mut builder = Client::builder();
				if let Some(x) = sub.get_one::<Cluster>(FLAG_COMMON_CLUSTER) {
					builder = builder.with_cluster(x.0.clone());
				}
				if let Some(x) = sub.get_one::<Context>(FLAG_COMMON_CONTEXT) {
					builder = builder.with_context(x.0.clone());
				}
				if let Some(x) = sub.get_one::<Namespace>(FLAG_COMMON_NAMESPACE) {
					builder = builder.with_namespace(x.0.clone());
				}

				let client = builder.build().await?;
				warn!("running client");

				let name: Name = if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_NAME) {
					Name::Strict(x.clone())
				} else if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_GENERATIVE_NAME) {
					Name::Generated(x.clone())
				} else {
					unreachable!()
				};

				let cmd = crate::cli::commands::run::Command::new(client, name);

				let cancel_token = CancellationToken::new();
				let run_cancel_token = cancel_token.clone();

				let join_handle = tokio::spawn(async move {
					info!("starting run command");
					match cmd.run(run_cancel_token).await {
						Ok(_) => info!("run finished OK"),
						Err(e) => error!("run command errored: {e}"),
					}
					info!("run command done");
				});

				tokio::select! {
					_ = tokio::signal::ctrl_c() => {
						 info!("received ctrl-c")
					},
					_ = join_handle => {
						info!("command finished")
					}
				}
				info!("cancelling cancel token");
				cancel_token.cancel();

				info!("all done!");
				Ok(())
			},
			Some((CMD_VERSION, _sub)) => Ok(()),
			_ => Ok(()),
		}
	}
}
