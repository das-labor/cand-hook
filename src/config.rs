use serde::Deserialize;
use std::net::SocketAddr;
use crate::hook::Hook;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server: SocketAddr,
    #[serde(rename = "hook")]
    pub hooks: Vec<Hook>
}