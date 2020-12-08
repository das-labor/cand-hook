use tokio::net::TcpStream;
use tokio::process::Command;
use std::fs;
use labctl::can::CanPacket;
use crate::config::Hook;
use tokio::time::{Instant, Duration, self};
use tokio::task;
use std::sync::Arc;
use tokio::sync::Mutex;

mod config;

fn args<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("cand-hook")
        .arg(
            clap::Arg::with_name("config")
                .long("config")
                .short("c")
                .required(true)
                .takes_value(true)
        )
}

#[tokio::main(flavor = "current_thread")]
async fn main() {

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let matches = args().get_matches();

    let mut config: config::Config = toml::from_str(
        &fs::read_to_string(&matches.value_of("config").unwrap()).unwrap()
    ).unwrap();

    let mut stream = TcpStream::connect(&config.server).await.unwrap();

    let cooldowns = Arc::new(Mutex::new(vec![None; config.hooks.len()]));

    log::info!("Connected to cand");

    loop {
        let p = labctl::can::read_packet_async(&mut stream).await.unwrap();
        log::trace!("Packet: {:?}", p);

        for (hook_num, hook) in config.hooks.iter_mut().enumerate() {
            if match_packet_against_config(&p, &hook) {
                {
                    let cooldown_lock = cooldowns.lock().await;
                    if let Some(last_activation) = cooldown_lock[hook_num] {
                        if let Some(cooldown) = hook.cooldown {
                            if last_activation + Duration::from_millis(cooldown) > Instant::now() {
                                log::debug!("Hook {:?} cooldown still pending", hook);
                                continue;
                            }
                        }
                    }
                }

                let hook = hook.clone();
                let p = p.clone();
                let cooldowns = cooldowns.clone();

                task::spawn(async move {
                    if let Some(delay) = hook.delay {
                        time::sleep(Duration::from_millis(delay)).await;
                    }

                    log::info!("Hook {:?} run", hook);
                    let mut cmd = Command::new(hook.run.get(0).unwrap());
                    cmd
                        .env("CAN_SRC_ADDR", format!("{:x}", p.src_addr))
                        .env("CAN_DST_ADDR", format!("{:x}", p.src_addr))
                        .env("CAN_SRC_PORT", format!("{:x}", p.src_addr))
                        .env("CAN_DST_PORT", format!("{:x}", p.src_addr))
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

                    cmd.spawn().unwrap();
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