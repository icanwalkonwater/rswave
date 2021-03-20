use crate::{
    led_controllers::LedController,
    net::{NetHandler, RemoteData},
    runners::{NoopRunner, Runner, RunnerEnum, SimpleBeatRunner, StandbyRunner},
    Opt,
};
use anyhow::Result;
use log::{debug, info};
use single_value_channel::Updater;
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};
use crate::runners::IntenseRunner;

#[derive(Debug, Copy, Clone)]
pub(crate) enum ControllerMessage {
    Standby,
    RandomRunner,
    Analysis { novelty: f64, is_beat: bool },
    Noop,
    Exit,
}

pub struct App<C: LedController + Send + 'static> {
    _opt: Opt,
    net: NetHandler,

    runner_thread: JoinHandle<()>,
    messenger: Updater<ControllerMessage>,

    _phantom: std::marker::PhantomData<C>,
}

impl<C: LedController + Send + 'static> App<C> {
    pub fn new(opt: Opt, controller: C) -> Result<Self> {
        let net = NetHandler::new(opt.port)?;
        let (runner_thread, messenger) = Self::make_controller_thread(opt, controller);

        Ok(Self {
            _opt: opt,
            net,
            runner_thread,
            messenger,
            _phantom: Default::default(),
        })
    }

    fn make_controller_thread(
        opt: Opt,
        mut controller: C,
    ) -> (JoinHandle<()>, Updater<ControllerMessage>) {
        let (mut receiver, updater) =
            single_value_channel::channel_starting_with(ControllerMessage::Noop);

        let handle = std::thread::Builder::new()
            .name("Led Runner Thread".into())
            .spawn(move || {
                let period = Duration::from_millis(opt.led_update_period);
                let mut runner: RunnerEnum = NoopRunner.into();

                loop {
                    let start = Instant::now();
                    match receiver.latest_mut() {
                        msg @ ControllerMessage::Standby => {
                            runner =
                                StandbyRunner::new(opt.standby_speed, opt.standby_reverse).into();
                            *msg = ControllerMessage::Noop;
                            info!("Runner: standby");
                        }
                        msg @ ControllerMessage::RandomRunner => {
                            runner = IntenseRunner::new().into();
                            *msg = ControllerMessage::Noop;
                            info!("Runner: common");
                        }
                        msg @ ControllerMessage::Analysis { .. } => {
                            if let ControllerMessage::Analysis { novelty, is_beat } = msg {
                                if *is_beat {
                                    runner.beat();
                                }
                                runner.novelty(*novelty);
                            }
                            *msg = ControllerMessage::Noop;
                        }
                        ControllerMessage::Exit => break,
                        ControllerMessage::Noop => {}
                    }

                    if runner.run_once() {
                        runner.display(&mut controller).unwrap();
                    }

                    // Wait for the rest of the period
                    std::thread::sleep(period - Instant::now().duration_since(start));
                }

                info!("Runner thread exit");
            })
            .expect("Failed to create runner thread !");
        debug!("Spawned runner thread !");

        (handle, updater)
    }

    pub fn run(&mut self) -> Result<()> {
        // Wait for remote
        if !self.net.is_connected() {
            self.messenger.update(ControllerMessage::Standby)?;
            self.net.wait_for_remote_blocking()?;
            self.net.handshake()?;
        }

        // Set a runner
        self.messenger.update(ControllerMessage::RandomRunner)?;

        // Wait for next packet
        loop {
            match self.net.recv()? {
                RemoteData::Analysis { novelty, is_beat } => {
                    self.messenger
                        .update(ControllerMessage::Analysis { novelty, is_beat })?;
                }
                RemoteData::Goodbye { .. } => {
                    // Ignore force flag
                    self.net.stop()?;
                    break;
                }
            }
        }

        // Remote has disconnected
        Ok(())
    }

    pub fn stop(self) -> Result<()> {
        self.messenger.update(ControllerMessage::Exit)?;
        self.runner_thread
            .join()
            .expect("Failed to join runner thread !");
        Ok(())
    }
}
