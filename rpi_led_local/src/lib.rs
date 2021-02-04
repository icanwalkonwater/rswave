use cichlid::ColorRGB;
use rs_ws281x::{ChannelBuilder, Controller, StripType, RawColor};

pub mod runners;

// Default: 800kHz
pub const LED_FREQ: u32 = 800_000;
// DO NOT USE 5 on RPi
pub const LED_DMA: i32 = 10;
pub const LED_COUNT: i32 = 5;
// GPIO18
pub const LED_PIN: i32 = 18;
// Don't change
pub const LED_CHANNEL: usize = 0;

pub const COLOR_OFF: RawColor = [0, 0, 0, 0];

pub trait ControllerExt {
    fn commit(&mut self) -> rs_ws281x::Result<()>;

    fn set_all(&mut self, color: ColorRGB) -> rs_ws281x::Result<()>;

    fn set_all_individual(&mut self, colors: &[ColorRGB]) -> rs_ws281x::Result<()>;

    fn set_all_raw(&mut self, color: RawColor) -> rs_ws281x::Result<()>;

    fn set_all_individual_raw(&mut self, colors: &[RawColor]) -> rs_ws281x::Result<()>;
}

impl ControllerExt for Controller {
    fn commit(&mut self) -> rs_ws281x::Result<()> {
        self.render()?;
        self.wait()
    }

    fn set_all(&mut self, color: ColorRGB) -> rs_ws281x::Result<()> {
        for led in self.leds_mut(LED_CHANNEL) {
            *led = [color.r, color.g, color.b, 0];
        }
        self.commit()
    }

    fn set_all_individual(&mut self, colors: &[ColorRGB]) -> rs_ws281x::Result<()> {
        assert!(colors.len() >= self.leds(LED_CHANNEL).len());
        for (i, led) in self.leds_mut(LED_CHANNEL).iter_mut().enumerate() {
            *led = [colors[i].r, colors[i].g, colors[i].b, 0];
        }
        self.commit()
    }

    fn set_all_raw(&mut self, color: RawColor) -> rs_ws281x::Result<()> {
        for led in self.leds_mut(LED_CHANNEL) {
            *led = color;
        }
        self.commit()
    }

    fn set_all_individual_raw(&mut self, colors: &[RawColor]) -> rs_ws281x::Result<()> {
        assert!(colors.len() >= self.leds(LED_CHANNEL).len());
        for (i, led) in self.leds_mut(LED_CHANNEL).iter_mut().enumerate() {
            *led = colors[i];
        }
        self.commit()
    }
}

pub fn create_led_controller() -> rs_ws281x::Result<Controller> {
    rs_ws281x::ControllerBuilder::new()
        .freq(LED_FREQ)
        .dma(LED_DMA)
        .channel(
            LED_CHANNEL,
            ChannelBuilder::new()
                .pin(LED_PIN)
                .count(LED_COUNT)
                .strip_type(StripType::Ws2811Gbr)
                .invert(false)
                .brightness(100)
                .build(),
        )
        .build()
}
