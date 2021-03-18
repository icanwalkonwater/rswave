use crate::led_controllers::LedController;
use cichlid::{ColorRGB, HSV};
use std::time::Instant;
use cichlid::prelude::RainbowFillSingleCycle;
use anyhow::Result;
use enum_dispatch::enum_dispatch;
use log::debug;

#[enum_dispatch]
pub enum RunnerEnum {
    NoopRunner,
    StandbyRunner,
    SimpleBeatRunner,
}

#[enum_dispatch(RunnerEnum)]
pub trait Runner {
    fn beat(&mut self);
    fn run_once(&mut self) -> bool;
    fn display<C: LedController>(&self, controller: &mut C) -> Result<()>;
}

// Noop runner
// <editor-fold>
pub struct NoopRunner;

impl Runner for NoopRunner {
    fn beat(&mut self) {
        // no-op
    }

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
    fn beat(&mut self) {
        // no-op
    }

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
                rainbow.iter_mut().rev().rainbow_fill_single_cycle(self.current_color.h);
            } else {
                rainbow.iter_mut().rainbow_fill_single_cycle(self.current_color.h);
            }
            controller.set_all_individual(&rainbow);
        } else {
            controller.set_all(self.current_color.to_rgb_rainbow());
        }

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
        self.current_color.h = self.current_color.h.wrapping_add(self.hue_increment);
        self.current_color.maximize_brightness();
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
// <editor-fold>
