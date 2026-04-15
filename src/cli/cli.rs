use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use anyhow::Result;
use clap::{
	Arg, Command,
	builder::{EnumValueParser, PathBufValueParser},
};
use tracing::warn;

use crate::k8s::pod::Name;
use crate::{
	cli::logging::{self, ConfigLevelFilter},
	k8s::client::Client,
};

const CMD_DEBUG: &str = "debug";
const CMD_EXEC: &str = "exec";
const CMD_RUN: &str = "run";
const CMD_VERSION: &str = "version";

const FLAG_KUBECONFIG_FILE: &str = "kubeconfig";
const FLAG_LOG_LEVEL: &str = "log-level";
const FLAG_PROFILES_FILE: &str = "profiles";

const CMD_RUN_FLAG_NAME: &str = "name";
const CMD_RUN_FLAG_GENERATIVE_NAME: &str = "generative-name";

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

impl Into<clap::builder::Str> for Cmd {
	fn into(self) -> clap::builder::Str {
		match self {
			Cmd::Debug => CMD_DEBUG.into(),
			Cmd::Exec => CMD_EXEC.into(),
			Cmd::Run => CMD_RUN.into(),
			Cmd::Version => CMD_VERSION.into(),
		}
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
				Arg::new(FLAG_KUBECONFIG_FILE)
					.long(FLAG_KUBECONFIG_FILE)
					.value_parser(PathBufValueParser::new()),
			)
			.arg(
				Arg::new("log-level")
					.long(FLAG_LOG_LEVEL)
					.default_value(logging::DEFAULT)
					.help("Set the log level")
					.long_help("Set the log level. 'trace' is the most verbose and 'off' the least verbose")
					.value_parser(EnumValueParser::<ConfigLevelFilter>::new()),
			)
			.arg(
				Arg::new(FLAG_PROFILES_FILE)
					.long(FLAG_PROFILES_FILE)
					.default_value(config_dir.into_os_string())
					.value_parser(PathBufValueParser::new()),
			)
			.arg_required_else_help(true)
			.subcommand(Command::new(CMD_DEBUG))
			.subcommand(Command::new(CMD_EXEC))
			.subcommand(
				Command::new(CMD_RUN)
					.arg(
						Arg::new(CMD_RUN_FLAG_GENERATIVE_NAME)
							.long(CMD_RUN_FLAG_GENERATIVE_NAME)
							.default_value("ksh-")
							.conflicts_with(CMD_RUN_FLAG_NAME),
					)
					.arg(Arg::new(CMD_RUN_FLAG_NAME).long(CMD_RUN_FLAG_NAME)),
			)
			.subcommand(Command::new(CMD_VERSION));

		Cli { root }
	}

	pub async fn parse(self) -> Result<()> {
		let matches = self.root.get_matches();

		// Configure logging first; let's figure out how to report back to the world.
		let log_level: &ConfigLevelFilter = matches
			.get_one::<ConfigLevelFilter>(FLAG_LOG_LEVEL)
			.unwrap_or_else(|| &logging::DEFAULT);
		logging::init(log_level);

		match matches.subcommand() {
			Some((CMD_DEBUG, _sub)) => Ok(()),
			Some((CMD_EXEC, _sub)) => Ok(()),
			Some((CMD_RUN, sub)) => {
				warn!("building builder");
				let builder = Client::builder();
				warn!("building client");
				let client = builder.build().await?;
				warn!("running client");

				let name: Name = if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_NAME) {
					Name::Strict(x.clone())
				} else if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_GENERATIVE_NAME) {
					Name::Generated(x.clone())
				} else {
					!unreachable!()
				};

				let cmd = crate::cli::commands::run::Command::new(client, name);

				cmd.run().await
			},
			Some((CMD_VERSION, _sub)) => Ok(()),
			_ => Ok(()),
		}
	}
}
