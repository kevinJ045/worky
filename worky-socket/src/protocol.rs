use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Request {
  Start {},
  Stop {},
  Restart {},
  Load {
    address: String,
    path: String,
    #[serde(default)]
    refresh: Option<bool>,
    #[serde(default)]
    name: Option<String>,
  },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
  pub status: String,
  pub message: Option<String>,
  pub error: Option<String>,
}
