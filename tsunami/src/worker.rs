use crate::net::{build_ipv4_packet, build_tcp_packet, create_send_sock, IP_HDR_LEN, TCP_HDR_LEN};
use crate::Port;
use anyhow::Result;
use pnet::packet::Packet;
use raw_socket::tokio::prelude::{Level, Name};
use std::{net::Ipv4Addr, sync::Arc};
use tokio::{
    sync::Semaphore,
    time::{sleep, Duration},
};

pub async fn inspect(
    dest: Ipv4Addr,
    port: Port,
    semaphore: Arc<Semaphore>,
    src_ip_addr: Ipv4Addr,
    nap_after_spawn: f64,
) -> Result<()> {
    if let Ok(_permit) = semaphore.acquire().await {
        let sock = create_send_sock()?;
        let mut ipv4_buf = vec![0u8; (IP_HDR_LEN + TCP_HDR_LEN) as usize];
        let mut ipv4_packet = build_ipv4_packet(&mut ipv4_buf, dest);
        let mut tcp_buf = vec![0u8; TCP_HDR_LEN as usize];
        let tcp_packet = build_tcp_packet(&mut tcp_buf, dest, port, src_ip_addr)?;

        ipv4_packet.set_payload(tcp_packet.packet());

        sock.set_sockopt(Level::IPV4, Name::IPV4_HDRINCL, &1i32)?;
        sock.send_to(ipv4_packet.packet(), (dest, port)).await?;

        sleep(Duration::from_secs_f64(nap_after_spawn / 1000.0)).await;
    }

    Ok(())
}
