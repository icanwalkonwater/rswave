use anyhow::{anyhow, Result};
use log::{debug, info, error};
use rswave_common::{
    packets::{
        AckPacket, DataMode, HelloPacket, NoveltyBeatsModePacket, NoveltyModePacket, SetModePacket,
    },
    rkyv::{
        archived_value, check_archive,
        de::deserializers::AllocDeserializer,
        ser::{serializers::WriteSerializer, Serializer},
        Deserialize, Serialize,
    },
    MAGIC,
};
use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};
use rswave_common::rkyv::Aligned;

#[derive(Debug)]
pub enum RemoteData {
    Analysis { novelty: f64, is_beat: bool },
    Goodbye { force: bool },
}

pub struct NetHandler {
    socket: UdpSocket,
    current_peer: Option<SocketAddr>,
    mode: DataMode,
    serialize_scratch: Option<Vec<u8>>,
    deserialize_scratch: Aligned<[u8; 128]>,
    is_stopped: bool,
}

impl NetHandler {
    pub fn new(port: u16) -> Result<Self> {
        let socket = UdpSocket::bind(SocketAddr::new([0, 0, 0, 0].into(), port))?;
        socket.set_nonblocking(false)?;

        Ok(Self {
            socket,
            current_peer: None,
            mode: DataMode::Novelty,
            serialize_scratch: None,
            deserialize_scratch: Aligned([0; 128]),
            is_stopped: false,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.current_peer.is_some()
    }

    pub fn wait_for_remote_blocking(&mut self) -> Result<()> {
        if self.current_peer.is_some() {
            debug!("Already connected, skip");
            return Ok(());
        }

        self.socket.set_nonblocking(true)?;
        let res = loop {
            match self.socket.recv_from(self.deserialize_scratch.as_mut()) {
                Ok((_, peer)) => {
                    self.current_peer = Some(peer);
                    self.socket.connect(peer)?;
                    break Ok(());
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => break Err(anyhow!(err)),
            }

            // Wait for a bit and retry
            std::thread::sleep(Duration::from_millis(500));
        };
        self.socket.set_nonblocking(false)?;

        info!("New peer: {}", self.current_peer.as_ref().unwrap());

        res
    }

    pub fn handshake(&mut self) -> Result<()> {
        // Hello has already been recv when waiting for a remote.

        debug!("Starting handshake...");

        // Hello
        let hello = unsafe { archived_value::<HelloPacket>(self.deserialize_scratch.as_ref(), 0) };
        let hello = hello.deserialize(&mut AllocDeserializer).unwrap();
        self.serialize_send(&hello)?;

        // SetMode
        self.socket.recv(self.deserialize_scratch.as_mut())?;
        let mode = unsafe { archived_value::<SetModePacket>(self.deserialize_scratch.as_ref(), 0) };
        let mode: SetModePacket = mode.deserialize(&mut AllocDeserializer).unwrap();
        debug!("Mode: {:?}", mode);
        self.mode = mode.mode;

        debug!("Handshake successful");

        Ok(())
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

    pub fn recv(&mut self) -> Result<RemoteData> {
        let len = self.socket.recv(self.deserialize_scratch.as_mut())?;

        let res = match self.mode {
            DataMode::Novelty => {
                let packet =
                    check_archive::<NoveltyModePacket>(&self.deserialize_scratch.as_ref()[..len], 0)
                        .map_err(|err| anyhow!("Check archive failed: {}", err))?;
                let packet: NoveltyModePacket = packet.deserialize(&mut AllocDeserializer)?;

                match packet {
                    NoveltyModePacket::Data(data) => Ok(RemoteData::Analysis {
                        novelty: data.value / data.peak,
                        is_beat: false,
                    }),
                    NoveltyModePacket::Goodbye(goodbye) if goodbye.magic == MAGIC => {
                        Ok(RemoteData::Goodbye {
                            force: goodbye.force,
                        })
                    }
                    _ => Err(anyhow!("Abort !")),
                }
            }
            DataMode::NoveltyBeats => {
                // TODO: don't deserialize, use the archive

                let packet =
                    check_archive::<NoveltyBeatsModePacket>(&self.deserialize_scratch.as_ref()[..len], 0)
                        .map_err(|err| anyhow!("Check archive failed: {}", err))?;
                let packet: NoveltyBeatsModePacket = packet.deserialize(&mut AllocDeserializer)?;

                match packet {
                    NoveltyBeatsModePacket::Data(data) => Ok(RemoteData::Analysis {
                        novelty: data.novelty.value / data.novelty.peak,
                        is_beat: data.beat,
                    }),
                    NoveltyBeatsModePacket::Goodbye(goodbye) if goodbye.magic == MAGIC => {
                        Ok(RemoteData::Goodbye {
                            force: goodbye.force,
                        })
                    }
                    _ => Err(anyhow!("Abort !")),
                }
            }
        };

        if res.is_ok() {
            let packet = AckPacket::Ok;
            self.serialize_send(&packet)?;
        } else {
            error!("Send ACK Abort");
            let packet = AckPacket::Abort;
            self.serialize_send(&packet)?;
            self.current_peer = None;
        }

        res
    }

    pub fn stop(&mut self) -> Result<()> {
        let ack = AckPacket::Quit;
        self.serialize_send(&ack)?;
        self.current_peer = None;
        self.is_stopped = true;

        Ok(())
    }
}

impl Drop for NetHandler {
    fn drop(&mut self) {
        if !self.is_stopped {
            eprintln!("Forgot to stop NetHandler !");
        }
    }
}
