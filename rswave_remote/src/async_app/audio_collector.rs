use log::debug;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use crate::async_app::errors::{ResultAudioCollector as Result, AudioCollectorError};
use cpal::{SampleRate, SampleFormat, Device, StreamConfig, SupportedStreamConfig};
use std::collections::VecDeque;
use crate::Opt;
use ringbuf::{RingBuffer, Consumer, Producer};
use tokio::sync::{oneshot};
use std::sync::Barrier;
use std::sync::Arc;
use tokio::task;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::task::JoinHandle;

pub struct AudioCollector {
    pub(crate) handle: task::JoinHandle<()>,
    pub(crate) stop_signal: oneshot::Sender<bool>,
    pub(crate) consumer: Consumer<f64>,
}

impl AudioCollector {
    pub fn new(opt: Opt) -> Result<AudioCollector> {
        let host = cpal::default_host();

        // Choose device
        let device = if let Some(hint) = opt.device_hint.as_ref() {
            host.input_devices()?
                .find(|device| device.name().map_or(false, |name| name.contains(hint)))
        } else {
            host.default_input_device()
        }.ok_or(AudioCollectorError::AudioDeviceNotFound)?;

        // Get config and check if we can handle it
        let config = device.default_input_config()?;
        if config.sample_rate() != SampleRate(44100) {
            return Err(AudioCollectorError::UnsupportedSampleRate).into();
        } else if config.channels() != 2 {
            return Err(AudioCollectorError::NotStereoDevice).into();
        }

        // Ring buffer for buffering (lol) samples, 2 times the sample size so we don't lose any
        let buffer = RingBuffer::new(opt.sample_size * 2);
        let (prod, cons) = buffer.split();
        // Vanilla barrier because cpal isn't async
        let buffer_barrier = Arc::new(Barrier::new(2));

        // Setup collecting task
        let (stop_signal, stop_recv) = oneshot::channel();

        let handle = tokio::task::spawn_blocking(move || {
            Self::run(device, config, opt.sample_size, prod, buffer_barrier, stop_recv);
        });

        // Ok we have everything
        Ok(AudioCollector {
            handle,
            stop_signal,
            consumer: cons,
        })
    }

    fn run(device: Device, config: SupportedStreamConfig, sample_size: usize, mut prod: Producer<f64>, buffer_barrier: Arc<Barrier>, mut stop: oneshot::Receiver<bool>) {
        let buffer_barrier_clone = buffer_barrier.clone();

        // Create reader here because it isn't `Send`
        let stream = match config.sample_format() {
            SampleFormat::I16 => {
                device.build_input_stream(&config.into(), move |data: &[i16], _| {
                    prod.push_iter(&mut data.iter().copied().map(|sample| sample as f64));
                    if prod.len() >= sample_size {
                        // Notify we are ready to send this batch
                        buffer_barrier_clone.wait();
                    }
                }, move |err| panic!(err))
            },
            SampleFormat::U16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _| {
                        prod.push_iter(
                            &mut data
                                .iter()
                                .copied()
                                .map(|sample| sample as f64 - u16::max_value() as f64 / 2.0),
                        );
                        if prod.len() >= sample_size {
                            // Notify we are ready to send this batch
                            buffer_barrier_clone.wait();
                        }
                    },
                    |err| panic!(err),
                )
            },
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        prod.push_iter(&mut data.iter().copied().map(|sample| sample as f64));
                        if prod.len() >= sample_size {
                            // Notify we are ready to send this batch
                            buffer_barrier_clone.wait();
                        }
                    },
                    |err| panic!(err),
                )
            },
        }.expect("Failed to create audio stream");

        stream.play().expect("Failed to start playing audio stream");
        while let Err(TryRecvError::Empty) = stop.try_recv() {
            // Wait for buffer to fill up
            buffer_barrier.wait();
        }
        stream.pause().expect("Failed to pause audio stream");
    }

    pub async fn stop(self) -> Result<()> {
        // If we can't send the signal it means that the
        // other has been dropped, so we don't need to await it
        if let Ok(_) = self.stop_signal.send(true) {
            self.handle.await.map_err(|_| AudioCollectorError::FailedToStopTask)?;
        }

        Ok(())
    }
}
