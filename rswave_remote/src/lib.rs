use structopt::StructOpt;

pub mod app;
pub mod audio;
pub mod net;
pub mod spotify;

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// Address to bind to
    #[structopt(short = "a", long)]
    pub address: Option<String>,

    /// A pattern to help take the right device
    /// Enabling this means disabling the manual selection of device
    #[structopt(short, long)]
    pub device_hint: Option<String>,

    /// Disable the TUI
    #[structopt(short = "t", long)]
    pub no_tui: bool,

    /// Disable ACK checks, this also means that if the remote goes down
    /// we won't be notified and will continue sending data
    #[structopt(long)]
    pub no_ack: bool,

    /// The spotify client ID
    // TODO: clap's requires() doesn't work
    #[structopt(long, env)]
    pub spotify_id: Option<String>,

    /// The spotify secret
    #[structopt(long, env)]
    pub spotify_secret: Option<String>,
}
