use structopt::StructOpt;

pub mod app;
pub mod audio;
pub mod spotify;

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// Address to bind to
    pub address: Option<String>,

    /// A pattern to help take the right device
    /// Enabling this means disabling the manual selection of device
    #[structopt(short, long)]
    pub device_hint: Option<String>,

    /// The spotify client ID
    /// Note: clap's requires() doesn't work
    #[structopt(long, env)]
    pub spotify_id: Option<String>,

    /// The spotify secret
    #[structopt(long, env)]
    pub spotify_secret: Option<String>,
}
