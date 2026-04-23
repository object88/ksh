use serde::Deserialize;

#[derive(Deserialize)]
pub struct Profile {
	command: Option<Vec<String>>,
}

impl Default for Profile {
	fn default() -> Self {
		Self { command: None }
	}
}
