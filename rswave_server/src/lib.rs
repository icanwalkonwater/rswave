use std::str::FromStr;
use structopt::StructOpt;
use anyhow::anyhow;

pub mod runners;
pub mod app;
pub mod led_controllers;
pub mod net;

#[derive(Copy, Clone, Debug, StructOpt)]
pub struct Opt {
    /// Port to use.
    #[structopt(short, long, default_value = "20200")]
    pub port: u16,

    /// Set overall brightness.
    #[structopt(short, long, default_value = "255")]
    pub brightness: u8,

    /// Reset the LED strip and exit.
    #[structopt(short, long)]
    pub reset: bool,

    /// Led strip type, will default to WS2811.
    #[structopt(short, long, default_value = "ws2811")]
    pub led_type: LedStripType,

    /// Amount of LEDs on the strip.
    #[structopt(short = "c", long)]
    pub led_count: usize,

    /// Delay during LED updates in milliseconds.
    #[structopt(long, default_value = "50")]
    pub led_update_period: u64,

    /// Controls the speed of the rainbow during the standby mode.
    #[structopt(long, default_value = "1.0")]
    pub standby_speed: f32,

    /// Reverse the rainbow effect of the standby runner.
    /// This effect will only be visible on addressable LED strips.
    #[structopt(long)]
    pub standby_reverse: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum LedStripType {
    Ws2811,
}

impl FromStr for LedStripType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ws2811" => Ok(Self::Ws2811),
            _ => Err(anyhow!("Unknown led strip type !")),
        }
    }
}
