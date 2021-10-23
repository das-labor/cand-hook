use tokio::net::TcpStream;
use std::fs;
use labctl::can::CanPacket;
use tokio::task;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::{UnboundedReceiver};
use tokio::sync::mpsc;
use tokio::io;

mod config;
mod control;
mod hook;

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

    let config: config::Config = toml::from_str(
        &fs::read_to_string(&matches.value_of("config").unwrap()).unwrap()
    ).unwrap();

    let stream = TcpStream::connect(&config.server).await.unwrap();

    let (mut stream, writer) = io::split(stream);

    let (sender, receiver) = mpsc::unbounded_channel();
    task::spawn(cand_writer_thread(writer, receiver));

    log::info!("Connected to cand");

    let mut hooks = hook::Hooks::new(config.hooks, sender);

    loop {
        let p = labctl::can::read_packet_async(&mut stream).await.unwrap();
        log::trace!("Packet: {:?}", p);

        hooks.process_hooks(&p).await;
    }
}

async fn cand_writer_thread<W: AsyncWrite + Unpin>(mut write: W, mut inbox: UnboundedReceiver<CanPacket>) {
    while let Some(msg) = inbox.recv().await {
        labctl::can::write_packet_to_cand_async(&mut write,  &msg).await.unwrap();
    }
}