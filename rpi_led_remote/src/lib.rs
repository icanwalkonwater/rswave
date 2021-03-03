use structopt::StructOpt;

pub mod app;
pub mod audio;

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// Address to bind to
    pub address: Option<String>,

    /// A pattern to help take the right device
    /// Enabling this means disabling the manual selection of device
    #[structopt(short, long)]
    pub device_hint: Option<String>,
}
