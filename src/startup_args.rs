use clap::Parser;

#[derive(Parser)]
#[clap(version = "1.0", author = "Valentin Buck<ivan.v.buck@student.fh-kiel.de>")]
pub struct StartupArgs{
    #[clap(short='b', long, default_value="0.0.0.0")]
    pub bind_address: String,
    #[clap(short='s', long, default_value="9001")]
    pub udp_send_port: u16,
    #[clap(short='r', long, default_value="9000")]
    pub udp_receive_port: u16,
    #[clap(short='w', long)]
    pub websocket_port: Option<u16>
}