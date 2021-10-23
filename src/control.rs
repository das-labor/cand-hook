
use labctl::can::{CanAddr, CanPacket};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;
use failure::Error;
use hex::FromHex;

type Result<T> = std::result::Result<T, Error>;

fn control_args() -> clap::App<'static, 'static> {
    clap::App::new("control")
        .setting(clap::AppSettings::NoBinaryName)
        .subcommand(
            clap::SubCommand::with_name("send")
                .help_short("send a can packet")
                .arg(clap::Arg::with_name("source")
                    .short("s")
                    .takes_value(true)
                    .long("source")
                    .default_value("00:00")
                )
                .arg(clap::Arg::with_name("DESTINATION")
                    .required(true)
                    .takes_value(true)
                )
                .arg(clap::Arg::with_name("PAYLOAD")
                    .required(true)
                    .takes_value(true)
                )
        )
}

pub async fn cand_control_fd_processor<R: AsyncRead + Unpin>(read: R, sender: UnboundedSender<CanPacket>) {
    let br = BufReader::new(read);
    let mut lines = br.lines();
    loop {
        if let Some(line) = lines.next_line().await.unwrap() {
            match process_control_message(&line, &sender).await {
                Ok(()) => (),
                Err(e) => {
                    println!("Hook Script control error: {}", e)
                }
            }
        } else {
            break;
        }
    }
}

pub async fn process_control_message(line: &str, sender: &UnboundedSender<CanPacket>) -> Result<()> {
    let args = control_args();
    let split = line.split(" ");
    let matches = args.get_matches_from_safe(split)?;
    match matches.subcommand() {
        ("send", Some(args)) => {
            let src: CanAddr = args.value_of("source").unwrap().parse()?;
            let dest: CanAddr = args.value_of("DESTINATION").unwrap().parse()?;
            let payload = Vec::from_hex(args.value_of("PAYLOAD").unwrap())?;

            let packet = CanPacket::new(src, dest, payload);
            sender.send(packet).unwrap();
        }
        (_, _) => {

        }
    }
    Ok(())
}