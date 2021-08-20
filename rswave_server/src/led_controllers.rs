use anyhow::Result;
use cichlid::ColorRGB;
#[cfg(feature = "controller_gpio")]
use rppal::gpio::{Gpio, OutputPin};
#[cfg(feature = "controller_ws2811")]
use rs_ws281x::{ChannelBuilder, ControllerBuilder, RawColor, StripType};

pub trait LedController {
    fn is_addressable_individually() -> bool;
    fn led_amount(&self) -> usize;
    fn set_all(&mut self, color: ColorRGB);
    fn set_all_individual(&mut self, colors: &[ColorRGB]);
    fn set_individual(&mut self, i: usize, color: ColorRGB);
    fn commit(&mut self) -> Result<()>;

    fn reset(&mut self) -> Result<()>;
}

// Controller WS2811
// <editor-fold>
#[cfg(feature = "controller_ws2811")]
pub struct ControllerWs2811 {
    inner: rs_ws281x::Controller,
}

#[cfg(feature = "controller_ws2811")]
unsafe impl Send for ControllerWs2811 {}

#[cfg(feature = "controller_ws2811")]
impl ControllerWs2811 {
    // Default: 800kHz
    const LED_FREQ: u32 = 800_000;
    // DO NOT USE 5 on RPi
    const LED_DMA: i32 = 10;
    // GPIO18
    const LED_PIN: i32 = 18;
    // Don't change
    const LED_CHANNEL: usize = 0;

    pub const COLOR_OFF: RawColor = [0, 0, 0, 0];

    pub fn new(led_count: usize, brightness: u8) -> Result<Self> {
        let inner = ControllerBuilder::new()
            .freq(Self::LED_FREQ)
            .dma(Self::LED_DMA)
            .channel(
                Self::LED_CHANNEL,
                ChannelBuilder::new()
                    .pin(Self::LED_PIN)
                    .count(led_count as i32)
                    .strip_type(StripType::Ws2811Gbr)
                    .invert(false)
                    .brightness(brightness)
                    .build(),
            )
            .build()?;

        Ok(Self { inner })
    }
}

#[cfg(feature = "controller_ws2811")]
impl LedController for ControllerWs2811 {
    fn is_addressable_individually() -> bool {
        true
    }

    fn led_amount(&self) -> usize {
        self.inner.leds(Self::LED_CHANNEL).len()
    }

    fn set_all(&mut self, color: ColorRGB) {
        let raw = [color.r, color.g, color.b, 0];
        for led in self.inner.leds_mut(Self::LED_CHANNEL) {
            *led = raw;
        }
    }

    fn set_all_individual(&mut self, colors: &[ColorRGB]) {
        for (i, led) in self
            .inner
            .leds_mut(Self::LED_CHANNEL)
            .iter_mut()
            .enumerate()
        {
            *led = [colors[i].r, colors[i].g, colors[i].b, 0];
        }
    }

    fn set_individual(&mut self, i: usize, color: ColorRGB) {
        self.inner.leds_mut(Self::LED_CHANNEL)[i] = [color.r, color.g, color.b, 0];
    }

    fn commit(&mut self) -> Result<()> {
        self.inner.render()?;
        self.inner.wait()?;
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        for led in self.inner.leds_mut(Self::LED_CHANNEL) {
            *led = Self::COLOR_OFF;
        }
        self.commit()
    }
}
// </editor-fold>

// GPIO Controller
// <editor-fold>
#[cfg(feature = "controller_gpio")]
pub struct ControllerGpio {
    gpio: Gpio,
    freq: f64,
    pins: [OutputPin; 3],
}

#[cfg(feature = "controller_gpio")]
impl ControllerGpio {
    pub fn new(freq: f64, red: u8, green: u8, blue: u8) -> Result<Self> {
        let gpio = Gpio::new()?;
        let red = gpio.get(red)?.into_output();
        let green = gpio.get(green)?.into_output();
        let blue = gpio.get(blue)?.into_output();
        let pins = [red, green, blue];

        let mut controller = Self { gpio, freq, pins };
        controller.reset()?;
        Ok(controller)
    }

    #[inline]
    fn red(&mut self) -> &mut OutputPin {
        &mut self.pins[0]
    }

    #[inline]
    fn green(&mut self) -> &mut OutputPin {
        &mut self.pins[1]
    }

    #[inline]
    fn blue(&mut self) -> &mut OutputPin {
        &mut self.pins[2]
    }
}

#[cfg(feature = "controller_gpio")]
impl LedController for ControllerGpio {
    fn is_addressable_individually() -> bool {
        false
    }

    fn led_amount(&self) -> usize {
        1
    }

    fn set_all(&mut self, color: ColorRGB) {
        let freq = self.freq;

        // The actual set_pwm_frequency function always returns Ok, so we can unwrap
        self.red()
            .set_pwm_frequency(freq, color.r as f64 / 255.0)
            .unwrap();
        self.green()
            .set_pwm_frequency(freq, color.g as f64 / 255.0)
            .unwrap();
        self.blue()
            .set_pwm_frequency(freq, color.b as f64 / 255.0)
            .unwrap();
    }

    fn set_all_individual(&mut self, _: &[ColorRGB]) {
        unimplemented!()
    }

    fn set_individual(&mut self, _: usize, color: ColorRGB) {
        self.set_all(color);
    }

    fn commit(&mut self) -> Result<()> {
        // no-op
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        for pin in self.pins.iter_mut() {
            pin.clear_pwm()?;
            pin.set_low();
        }
        Ok(())
    }
}
// <editor-fold>
