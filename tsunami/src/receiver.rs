use crate::{
    net::{create_recv_sock, IP_HDR_LEN},
    Message, Port, PortInfo, PortStatus,
};
use anyhow::{bail, Result};
use pnet::packet::tcp::{TcpFlags, TcpPacket};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::{sync::mpsc::Sender, time::timeout};

const SYNACK: u8 = TcpFlags::SYN | TcpFlags::ACK;
const RSTACK: u8 = TcpFlags::RST | TcpFlags::ACK;

pub async fn receive(
    combined: HashSet<Port>,
    tx: Sender<Message>,
    max_retries: usize,
) -> Result<()> {
    let sock = create_recv_sock()?;
    let mut buf = [0u8; 576];

    let mut status = combined
        .iter()
        .map(|port| {
            (
                *port,
                PortInfo {
                    status: PortStatus::NotInspected,
                    retried: 0,
                },
            )
        })
        .collect::<HashMap<u16, PortInfo>>();

    tx.send(Message::Payload(status.keys().copied().collect()))
        .await?;

    status.iter_mut().for_each(|(_, info)| info.retried += 1);

    loop {
        let (_bytes_recvd, _ip_addr) =
            match timeout(Duration::from_millis(300), sock.recv_from(&mut buf)).await {
                Ok(result) => {
                    let (bytes, ip) = result.unwrap();

                    (Some(bytes), Some(ip))
                }
                Err(_) => {
                    let not_inspected = status
                        .iter()
                        .filter(|(_, info)| {
                            info.status == PortStatus::NotInspected && info.retried < max_retries
                        })
                        .map(|(port, _)| port.to_owned())
                        .collect::<Vec<u16>>();

                    if not_inspected.is_empty() {
                        tx.send(Message::Break).await?;
                        break;
                    } else {
                        status
                            .iter_mut()
                            .filter(|(port, _)| not_inspected.contains(port))
                            .for_each(|(_, info)| {
                                if info.retried < max_retries {
                                    info.retried += 1
                                }
                            });
                        tx.send(Message::Payload(not_inspected)).await?;
                    }

                    (None, None)
                }
            };

        let tcp_packet = match TcpPacket::new(&buf[IP_HDR_LEN as usize..]) {
            Some(packet) => packet,
            None => bail!("couldn't make tcp packet"),
        };

        let port = tcp_packet.get_source();

        match tcp_packet.get_flags() {
            SYNACK => {
                if let Some(info) = status.get_mut(&port) {
                    info.status = PortStatus::Open;
                    println!("{port}: open");
                }
            }
            RSTACK => {
                if let Some(info) = status.get_mut(&port) {
                    info.status = PortStatus::Closed;
                }
            }
            _ => {}
        }
    }

    let closed_count = status
        .iter()
        .filter(|(_, info)| info.status == PortStatus::Closed)
        .count();

    println!("ports closed: {closed_count}");

    status
        .iter_mut()
        .filter(|(_, info)| info.retried >= max_retries)
        .for_each(|(_, info)| info.status = PortStatus::Filtered);

    let filtered_count = status
        .iter()
        .filter(|(_, info)| info.status == PortStatus::Filtered)
        .count();

    println!("ports filtered: {filtered_count}");

    let retried_more_than_once_count = status.iter().filter(|(_, info)| info.retried > 1).count();

    println!("ports retried more than once: {retried_more_than_once_count}");

    Ok(())
}
