pub mod logging;

mod builder;
mod commands;
mod constants;

use std::fmt::{Display, Formatter};
use std::future::Future;
use std::path::PathBuf;

use anyhow::Result;
use clap::{
	Arg, Command,
	builder::{EnumValueParser, PathBufValueParser},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
	builder::{args::CommandExt, client::BuilderExt},
	logging::ConfigLevelFilter,
};
use ksh::k8s::{client::Client, pod::Name};

const CMD_DEBUG: &str = "debug";
const CMD_EXEC: &str = "exec";
const CMD_RUN: &str = "run";
const CMD_VERSION: &str = "version";

// These flags are used on all levels of the app
const FLAG_GLOBAL_LOG_LEVEL: &str = "log-level";
const FLAG_GLOBAL_PROFILES_FILE: &str = "profiles";

const CMD_RUN_FLAG_NAME: &str = "name";
const CMD_RUN_FLAG_GENERATIVE_NAME: &str = "generative-name";

pub trait Runnable {
	fn run(&self, cancel: CancellationToken) -> impl Future<Output = Result<()>> + Send;
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
					.build_common_flags()
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
				let builder = Client::builder().with_common_flags(sub);
				let client = builder.build().await?;
				warn!("running client");

				let name: Name = if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_NAME) {
					Name::Strict(x.clone())
				} else if let Some(x) = sub.get_one::<String>(CMD_RUN_FLAG_GENERATIVE_NAME) {
					Name::Generated(x.clone())
				} else {
					unreachable!()
				};

				let cmd = crate::commands::run::Command::new(client, name);

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
