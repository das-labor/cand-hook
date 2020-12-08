use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server: SocketAddr,
    #[serde(rename = "hook")]
    pub hooks: Vec<Hook>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hook {
    #[serde(rename = "src-addr")]
    pub src_addr: Option<u8>,
    #[serde(rename = "src-port")]
    pub src_port: Option<u8>,
    #[serde(rename = "dst-addr")]
    pub dst_addr: Option<u8>,
    #[serde(rename = "dst-port")]
    pub dst_port: Option<u8>,
    pub payload: Option<Vec<u8>>,
    pub run: Vec<String>,
    pub cooldown: Option<u64>,
    pub delay: Option<u64>
}