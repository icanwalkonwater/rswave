use structopt::StructOpt;

pub mod app;
pub mod audio;
pub mod net;
pub mod async_app;
pub mod spotify;

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// Address to bind to.
    #[structopt(short = "a", long)]
    pub address: Option<String>,

    /// A pattern to help take the right device.
    /// Enabling this means disabling the manual selection of device.
    #[structopt(short, long)]
    pub device_hint: Option<String>,

    /// Sample size for audio.
    /// It isn't recommended to change it at all but if you
    /// do so make sure that it is a power of two.
    #[structopt(short, long, default_value = "2048")]
    pub sample_size: usize,

    /// Compression to use for the logarithmic compression of the spectrum.
    /// The higher the value, the more importance the little values will get.
    #[structopt(long, default_value = "1000")]
    pub spectrum_compression: f64,

    /// Buffer size for the novelty curve.
    /// This is mainly to have a pretty curve to look at.
    /// However it must always be superior or equal to the short term
    /// novelty size.
    #[structopt(long, default_value = "200")]
    pub novelty_size: usize,

    /// Short term novelty size.
    /// Only this many samples will be used to compute the data to send.
    /// Decreasing it may increase the overall sensitivity of the system.
    #[structopt(long, default_value = "50")]
    pub novelty_size_st: usize,

    /// Disable the TUI.
    #[structopt(short = "t", long)]
    pub no_tui: bool,

    /// Disable ACK checks, this also means that if the remote goes down
    /// we won't be notified and will continue sending data
    #[structopt(long)]
    pub no_ack: bool,

    /// Maximum interval between calls to the spotify API to check for
    /// the currently playing track.
    /// Too much requests will be rate limited so stay reasonable.
    #[structopt(long, default_value = "5")]
    pub spotify_refresh_interval: f32,

    /// The spotify client ID.
    // TODO: clap's requires() doesn't work
    #[structopt(long, env)]
    pub spotify_id: Option<String>,

    /// The spotify secret.
    #[structopt(long, env)]
    pub spotify_secret: Option<String>,

    /// Don't use the cached token for authentication,
    /// instead ask the user to log in again.
    #[structopt(long)]
    pub spotify_auth_fresh: bool,
}
