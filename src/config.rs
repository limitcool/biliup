use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    // #[serde(rename = "GoogleToken")]
    pub google_token: String,
    pub streamers: HashMap<String, Streamers>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Streamers {
    pub url: String,
    pub test: String,
}
