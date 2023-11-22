use crate::{
    net::{create_recv_sock, IP_HDR_LEN},
    Message,
};
use anyhow::{bail, Result};
use pnet::packet::tcp::{TcpFlags, TcpPacket};
use std::{collections::HashSet, time::Duration};
use tokio::{sync::mpsc::Sender, time::timeout};

const SYNACK: u8 = TcpFlags::SYN | TcpFlags::ACK;
const RSTACK: u8 = TcpFlags::RST | TcpFlags::ACK;

pub async fn receive(combined: HashSet<u16>, tx: Sender<Message>) -> Result<()> {
    let sock = create_recv_sock()?;
    let mut buf = [0u8; 576];

    tx.send(Message::Payload(combined.clone())).await?;

    let mut recvd = HashSet::new();

    loop {
        let (_bytes_recvd, _ip_addr) =
            match timeout(Duration::from_millis(300), sock.recv_from(&mut buf)).await {
                Ok(result) => {
                    let (bytes, ip) = result.unwrap();

                    (Some(bytes), Some(ip))
                }
                Err(_) => {
                    let diff = combined.difference(&recvd);
                    if diff.clone().next().is_none() {
                        tx.send(Message::Break).await?;
                        break;
                    } else {
                        tx.send(Message::Payload(
                            diff.clone().map(|x| x.to_owned()).collect(),
                        ))
                        .await?;
                    }

                    (None, None)
                }
            };

        let tcp_packet = match TcpPacket::new(&buf[IP_HDR_LEN as usize..]) {
            Some(packet) => packet,
            None => bail!("couldn't make tcp packet"),
        };

        let port = tcp_packet.get_source();

        recvd.insert(port);

        match tcp_packet.get_flags() {
            SYNACK => println!("{port}: open"),
            RSTACK => println!("{port}: closed"),
            _ => {}
        }
    }

    Ok(())
}
