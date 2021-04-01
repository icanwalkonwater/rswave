use crate::led_controllers::LedController;
use anyhow::Result;
use cichlid::{prelude::RainbowFillSingleCycle, ColorRGB, HSV};
use enum_dispatch::enum_dispatch;
use log::debug;
use std::time::Instant;

#[enum_dispatch]
pub enum RunnerEnum {
    NoopRunner,
    StandbyRunner,
    WhiteRunner,
    SimpleBeatRunner,
    EpilepsyRunner,
}

#[enum_dispatch(RunnerEnum)]
pub trait Runner {
    fn beat(&mut self) {}
    fn novelty(&mut self, _novelty: f64) {}
    fn run_once(&mut self) -> bool;
    fn display<C: LedController>(&self, controller: &mut C) -> Result<()>;
}

fn hue_randomizer(mut color: HSV) -> HSV {
    let min = color.h.wrapping_sub(25);
    let max = color.h.wrapping_add(25);
    let range = if min < max { min..max } else { max..min };
    color.h = loop {
        let hue = rand::random::<u8>();
        if !range.contains(&hue) {
            break hue;
        }
    };
    color
}

// Noop runner
// <editor-fold>
pub struct NoopRunner;

impl Runner for NoopRunner {
    fn run_once(&mut self) -> bool {
        // no-op
        false
    }

    fn display<C: LedController>(&self, _: &mut C) -> Result<()> {
        // no-op
        Ok(())
    }
}
// </editor-fold>

// Standby runner
// <editor-fold>
pub struct StandbyRunner {
    current_color: HSV,
    speed: f32,
    reverse: bool,
    last_update: Instant,
}

impl StandbyRunner {
    pub fn new(speed: f32, reverse: bool) -> Self {
        debug!("Create standby runner with speed {}", speed);
        Self {
            current_color: HSV::new(0, 255, 255),
            speed,
            reverse,
            last_update: Instant::now(),
        }
    }
}

impl Runner for StandbyRunner {
    fn run_once(&mut self) -> bool {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_update).as_secs_f32();

        let hue_shift = (delta_time * self.speed * u8::MAX as f32) as u8;
        self.current_color.h = self.current_color.h.wrapping_add(hue_shift);
        self.current_color.maximize_brightness();

        self.last_update = now;
        true
    }

    fn display<C: LedController>(&self, controller: &mut C) -> Result<()> {
        if C::is_addressable_individually() {
            let mut rainbow = vec![ColorRGB::default(); controller.led_amount()];
            if self.reverse {
                rainbow
                    .iter_mut()
                    .rev()
                    .rainbow_fill_single_cycle(self.current_color.h);
            } else {
                rainbow
                    .iter_mut()
                    .rainbow_fill_single_cycle(self.current_color.h);
            }
            controller.set_all_individual(&rainbow);
        } else {
            controller.set_all(self.current_color.to_rgb_rainbow());
        }

        controller.commit()
    }
}
// </editor-fold>

// White runner (for debug purposes mainly)
// <editor-fold>
pub struct WhiteRunner {
    value: f32,
    gravity: f32,
    last_update: Instant,
}

impl WhiteRunner {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            gravity: 500.0,
            last_update: Instant::now(),
        }
    }
}

impl Runner for WhiteRunner {
    fn beat(&mut self) {
        self.value = 255.0;
    }

    fn run_once(&mut self) -> bool {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_update).as_secs_f32();
        self.value = (self.value - self.gravity * delta_time).max(0.0);
        self.last_update = now;
        true
    }

    fn display<C: LedController>(&self, controller: &mut C) -> Result<()> {
        let col = self.value as u8;
        controller.set_all(ColorRGB::new(col, col, col));
        controller.commit()
    }
}
// </editor-fold>

// Simple beat runner
// <editor-fold>
pub struct SimpleBeatRunner {
    current_color: HSV,
    hue_increment: u8,
    need_update: bool,
}

impl SimpleBeatRunner {
    pub fn new() -> Self {
        Self {
            current_color: HSV::new(0, 255, 255),
            hue_increment: u8::MAX / 6,
            need_update: true,
        }
    }
}

impl Runner for SimpleBeatRunner {
    fn beat(&mut self) {
        self.current_color.h = loop {
            let new_hue = rand::random();
            if (new_hue as i16 - self.current_color.h as i16).abs() > 50 {
                break new_hue;
            }
        };
        self.need_update = true;
    }

    fn run_once(&mut self) -> bool {
        if self.need_update {
            self.need_update = false;
            true
        } else {
            false
        }
    }

    fn display<C: LedController>(&self, controller: &mut C) -> Result<()> {
        controller.set_all(self.current_color.to_rgb_rainbow());
        controller.commit()
    }
}
// </editor-fold>

// Epilepsy runner
// <editor-fold>
pub struct EpilepsyRunner {
    current_color: HSV,
    gravity: f32,
    last_update: Instant,
}

impl EpilepsyRunner {
    pub fn new() -> Self {
        Self {
            current_color: HSV::new(0, 255, 255),
            gravity: 150.0,
            last_update: Instant::now(),
        }
    }
}

impl Runner for EpilepsyRunner {
    fn beat(&mut self) {
        self.current_color.maximize_brightness();
    }

    fn novelty(&mut self, novelty: f64) {
        if novelty > 0.3 {
            self.current_color = hue_randomizer(self.current_color);
        }
    }

    fn run_once(&mut self) -> bool {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_update).as_secs_f32();
        let brightness = (self.current_color.v as f32 / 2.55 - self.gravity * delta_time).max(0.0);
        self.current_color.v = ((brightness * 2.55) as u8).max(20);

        self.last_update = now;
        true
    }

    fn display<C: LedController>(&self, controller: &mut C) -> Result<()> {
        controller.set_all(self.current_color.to_rgb_spectrum());
        controller.commit()
    }
}

// </editor-fold>
