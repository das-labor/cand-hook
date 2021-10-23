use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use labctl::can::CanPacket;
use tokio::process::Command;
use tokio::{time, task};
use tokio::time::{Duration, Instant};
use crate::control;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;

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

pub struct Hooks {
    hooks: Vec<Hook>,
    cooldowns: Arc<Mutex<Vec<Option<Instant>>>>,
    sender: UnboundedSender<CanPacket>
}

impl Hooks {
    pub fn new(hooks: Vec<Hook>, sender: UnboundedSender<CanPacket>) -> Hooks {
        let cooldowns = Arc::new(Mutex::new(vec![None; hooks.len()]));
        log::trace!("Loaded hooks: {:?}", hooks);
        Hooks {
            hooks,
            cooldowns,
            sender
        }
    }

    pub async fn process_hooks(&mut self, packet: &CanPacket) {
        for (hook_num, hook) in self.hooks.iter_mut().enumerate() {
            if match_packet_against_config(&packet, &hook) {

                let hook = hook.clone();
                let p = packet.clone();
                let cooldowns = self.cooldowns.clone();
                let sender = self.sender.clone();

                task::spawn(async move {
                    if let Some(delay) = hook.delay {
                        log::info!("Pending hook execution in {} ms", delay);
                        time::sleep(Duration::from_millis(delay)).await;
                    }

                    {
                        let cooldown_lock = cooldowns.lock().await;
                        if let Some(last_activation) = cooldown_lock[hook_num] {
                            if let Some(cooldown) = hook.cooldown {
                                if last_activation + Duration::from_millis(cooldown) > Instant::now() {
                                    log::debug!("Hook {:?} cooldown still pending", hook);
                                    return;
                                }
                            }
                        }
                    }

                    log::info!("Hook {:?} run", hook);
                    let mut cmd = Command::new(hook.run.get(0).unwrap());
                    cmd
                        .env("CAN_SRC_ADDR", format!("{:x}", p.src_addr))
                        .env("CAN_DST_ADDR", format!("{:x}", p.dest_addr))
                        .env("CAN_SRC_PORT", format!("{:x}", p.src_port))
                        .env("CAN_DST_PORT", format!("{:x}", p.dest_port))
                        .env(
                            "CAN_PAYLOAD",
                            p.payload
                                .iter()
                                .map(|x| format!("{:x}", x))
                                .collect::<String>()
                        );
                    for arg in hook.run.iter().skip(1) {
                        cmd.arg(arg);
                    }
                    cmd.stdout(Stdio::piped());

                    let res = cmd.spawn().unwrap();
                    task::spawn(control::cand_control_fd_processor(res.stdout.unwrap(), sender));

                    cooldowns.lock().await[hook_num] = Some(Instant::now())
                });
            }
        }
    }
}

fn match_packet_against_config(p: &CanPacket, h: &Hook) -> bool {
    if let Some(src_addr) = h.src_addr {
        if src_addr != p.src_addr {
            return false;
        }
    }

    if let Some(dst_addr) = h.dst_addr {
        if dst_addr != p.dest_addr {
            return false;
        }
    }

    if let Some(src_port) = h.src_port {
        if src_port != p.src_port {
            return false;
        }
    }

    if let Some(dst_port) = h.dst_port {
        if dst_port != p.dest_port {
            return false;
        }
    }

    if let Some(payload) = &h.payload {
        if payload != &p.payload {
            return false;
        }
    }

    true
}