use crate::{audio::AudioProcessor, spotify::SpotifyTracker};
use anyhow::{anyhow, Result};
use rswave_common::{
    packets::{
        AckPacket, DataMode, GoodbyeData, HelloPacket, NoveltyBeatsModeData,
        NoveltyBeatsModePacket, NoveltyModeData, NoveltyModePacket, SetModePacket,
    },
    rkyv::{
        archived_value,
        ser::{serializers::WriteSerializer, Serializer},
        Aligned, Archived, Serialize,
    },
    MAGIC,
};
use std::net::UdpSocket;

pub struct NetHandler {
    socket: UdpSocket,
    mode: DataMode,
    stopped: bool,

    serialize_scratch: Option<Vec<u8>>,
    deserialize_scratch: Aligned<[u8; 128]>,
}

impl NetHandler {
    pub fn new(address: &str) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(false)?;
        socket.connect(address)?;

        Ok(Self {
            socket,
            mode: DataMode::Novelty,
            stopped: false,
            serialize_scratch: Some(Vec::new()),
            deserialize_scratch: Aligned([0; 128]),
        })
    }

    fn serialize_send(&mut self, item: &impl Serialize<WriteSerializer<Vec<u8>>>) -> Result<()> {
        if let Some(scratch) = &mut self.serialize_scratch {
            scratch.clear();
        } else {
            self.serialize_scratch = Some(Vec::new());
        }

        let mut serializer = WriteSerializer::new(self.serialize_scratch.take().unwrap());
        serializer.serialize_value(item)?;

        let buff = serializer.into_inner();
        self.socket.send(&buff)?;

        self.serialize_scratch.replace(buff);
        Ok(())
    }

    pub fn handshake(&mut self, mode: DataMode) -> Result<()> {
        let hello = HelloPacket::default();

        self.serialize_send(&hello)?;

        self.socket
            .recv(self.deserialize_scratch.as_mut())
            .expect("Failed to receive");
        let remote_hello =
            unsafe { archived_value::<HelloPacket>(self.deserialize_scratch.as_mut(), 0) };

        if hello.magic != remote_hello.magic || hello.random != remote_hello.random {
            return Err(anyhow!("Handshake failed !"));
        }

        self.mode = mode;
        let mode = SetModePacket { mode };
        self.serialize_send(&mode)?;
        Ok(())
    }

    pub fn send_current_data(
        &mut self, audio: &AudioProcessor, spotify: Option<&SpotifyTracker>, no_ack: bool,
    ) -> Result<()> {
        let novelty_data = NoveltyModeData {
            value: audio.novelty(),
            peak: audio.novelty_peak_short_term(),
        };

        match self.mode {
            DataMode::Novelty => {
                let packet = NoveltyModePacket::Data(novelty_data);
                self.serialize_send(&packet)?;
            }
            DataMode::NoveltyBeats => {
                let packet = NoveltyBeatsModePacket::Data(NoveltyBeatsModeData {
                    novelty: novelty_data,
                    beat: spotify.as_ref().map(|s| s.is_beat()).unwrap_or(false),
                });
                self.serialize_send(&packet)?;
            }
        }

        /*if !no_ack {
            self.check_ack()?;
        }*/

        Ok(())
    }

    /*fn check_ack(&mut self) -> Result<()> {
        self.socket.recv(self.deserialize_scratch.as_mut())?;
        let archived = unsafe { archived_value::<AckPacket>(self.deserialize_scratch.as_ref(), 0) };
        if let ArchivedAckPacket::Ok = archived {
            Ok(())
        } else {
            Err(anyhow!("Server quit/abort !"))
        }
    }*/

    pub fn stop(&mut self, force: bool) -> Result<()> {
        match self.mode {
            DataMode::Novelty => {
                let packet = NoveltyModePacket::Goodbye(GoodbyeData {
                    magic: MAGIC,
                    force,
                });
                self.serialize_send(&packet)?;
            }
            DataMode::NoveltyBeats => {
                let packet = NoveltyBeatsModePacket::Goodbye(GoodbyeData {
                    magic: MAGIC,
                    force,
                });
                self.serialize_send(&packet)?;
            }
        }

        self.socket.recv(self.deserialize_scratch.as_mut())?;
        let archived: &Archived<AckPacket> =
            unsafe { archived_value::<AckPacket>(self.deserialize_scratch.as_ref(), 0) };
        if let Archived::<AckPacket>::Quit = archived {
            self.stopped = true;
            Ok(())
        } else {
            println!("{:?}", archived);
            Err(anyhow!("Something went wrong somewhere !"))
        }
    }
}

impl Drop for NetHandler {
    fn drop(&mut self) {
        if !self.stopped {
            eprintln!("Forgot to stop NetHandler !");
        }
    }
}
