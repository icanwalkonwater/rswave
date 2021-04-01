use crate::async_app::errors::ResultNet as Result;
use rswave_common::rkyv::Aligned;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

pub struct NetHandler {
    serialize_scratch: Option<Box<[u8]>>,
    deserialize_scratch: Aligned<[u8; 128]>,
}

impl NetHandler {
    pub async fn new(address: &str) -> Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::new(0, 0, 0, 0), 0))
            .await
            .expect("Invalid socket addr, should never happen");
        let (mut recv, send) = socket.split();

        let recv_task = tokio::task::spawn(async move {
            let mut deserialize_scratch = Aligned([0; 128]);

            loop {
                let len = recv
                    .recv(deserialize_scratch.as_mut())
                    .await
                    .expect("Recv call failed, should never happen");
            }
        });

        Ok(Self {
            serialize_scratch: None,
            deserialize_scratch: Aligned([0; 128]),
        })
    }

    fn start(&mut self) {}

    async fn handle_recv() {}
}
