use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Result};
use pnet::packet::{
    ip::IpNextHeaderProtocols,
    ipv4::{Ipv4Flags, MutableIpv4Packet},
    tcp::{MutableTcpPacket, TcpFlags, TcpPacket},
    Packet,
};
use raw_socket::{
    tokio::prelude::{Level, Name, RawSocket},
    Domain, Protocol, Type,
};
use structopt::StructOpt;
use tokio::{sync::Semaphore, time::timeout};

const IP_HDR_LEN: u16 = 20;
const TCP_HDR_LEN: u16 = 20;
const IPPROTO_RAW: i32 = 255;

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(opts.target, &opts.ports, &opts.ranges).await {
        eprintln!("tsunami: {:?}", e);
    }
}

async fn run(target: Ipv4Addr, ports: &[u16], ranges: &[PortRange]) -> Result<()> {
    let mut tasks = vec![];

    let receiver = tokio::spawn(receive());

    let id_table = Arc::new(Mutex::new(HashMap::new()));
    let semaphore = Arc::new(Semaphore::new(512));

    let mut combined = HashSet::new();

    for port in ports {
        combined.insert(*port);
    }

    for range in ranges {
        for port in range.start..range.end {
            combined.insert(port);
        }
    }

    for port in combined {
        tasks.push(tokio::spawn(worker(
            target,
            port,
            semaphore.clone(),
            id_table.clone(),
        )));
    }

    for task in tasks {
        task.await??;
    }

    receiver.await??;

    Ok(())
}

async fn worker(
    dest: Ipv4Addr,
    port: u16,
    semaphore: Arc<Semaphore>,
    _id_table: Arc<Mutex<HashMap<u16, u16>>>,
) -> Result<()> {
    if let Ok(permit) = semaphore.acquire().await {
        let sock = RawSocket::new(
            Domain::ipv4(),
            Type::raw(),
            Protocol::from(IPPROTO_RAW).into(),
        )?;
        let id = rand::random::<u16>();
        let mut ipv4_buf = vec![0u8; (IP_HDR_LEN + TCP_HDR_LEN) as usize];
        let mut ipv4_packet = build_ipv4_packet(&mut ipv4_buf, dest, id);
        let mut tcp_buf = vec![0u8; TCP_HDR_LEN as usize];
        let tcp_packet = build_tcp_packet(&mut tcp_buf, dest, port);

        ipv4_packet.set_payload(tcp_packet.packet());

        sock.set_sockopt(Level::IPV4, Name::IPV4_HDRINCL, &1i32)?;
        sock.send_to(ipv4_packet.packet(), (dest, port)).await?;

        tokio::time::sleep(Duration::from_millis(20)).await;

        drop(permit);
    }

    Ok(())
}

async fn receive() -> Result<()> {
    let sock = create_sock()?;
    let mut buf = [0u8; 576];

    loop {
        let (_bytes_recvd, _ip_addr) =
            match timeout(Duration::from_secs(1), sock.recv_from(&mut buf)).await {
                Ok(result) => result.unwrap(),
                Err(_) => {
                    break;
                }
            };

        let tcp_packet = match TcpPacket::new(&buf[IP_HDR_LEN as usize..]) {
            Some(packet) => packet,
            None => bail!("couldn't make tcp packet"),
        };

        let port = tcp_packet.get_source();

        match tcp_packet.get_flags() {
            18 => println!("{port}: open"),
            20 => println!("{port}: closed"),
            _ => {}
        }
    }

    Ok(())
}

fn build_ipv4_packet(buf: &mut [u8], dest: Ipv4Addr, id: u16) -> MutableIpv4Packet {
    use pnet::packet::ipv4::checksum;

    let mut packet = MutableIpv4Packet::new(buf).unwrap();

    packet.set_version(4);
    packet.set_ttl(u8::MAX);
    packet.set_header_length(5); /* n * 32 bits. */

    /* We are setting the identification field to the TTL
     * that we later use to map responses back to correct hops. */
    packet.set_identification(id);
    packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    packet.set_destination(dest);
    packet.set_flags(Ipv4Flags::DontFragment);
    packet.set_total_length(IP_HDR_LEN + TCP_HDR_LEN);
    packet.set_checksum(checksum(&packet.to_immutable()));

    packet
}

fn build_tcp_packet(buf: &mut [u8], destination: Ipv4Addr, port: u16) -> MutableTcpPacket {
    use pnet::packet::tcp::ipv4_checksum;

    let mut packet = MutableTcpPacket::new(buf).unwrap();
    packet.set_source(0x1337_u16);
    packet.set_destination(port);
    packet.set_sequence(rand::random::<u32>());
    packet.set_data_offset(5);
    packet.set_flags(TcpFlags::SYN);
    packet.set_window(0x7110_u16);
    packet.set_checksum(ipv4_checksum(
        &packet.to_immutable(),
        &Ipv4Addr::new(192, 168, 1, 64),
        &destination,
    ));

    packet
}

fn create_sock() -> Result<Arc<RawSocket>> {
    match RawSocket::new(Domain::ipv4(), Type::raw(), Protocol::tcp().into()) {
        Ok(sock) => Ok(Arc::new(sock)),
        Err(_) => bail!("couldn't create the socket"),
    }
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    target: Ipv4Addr,

    #[structopt(short, long)]
    ports: Vec<u16>,

    #[structopt(short, long)]
    ranges: Vec<PortRange>,
}

#[derive(Debug, Clone, Copy)]
struct PortRange {
    start: u16,
    end: u16,
}

impl From<Vec<u16>> for PortRange {
    fn from(value: Vec<u16>) -> Self {
        Self {
            start: value[0],
            end: value[1],
        }
    }
}

impl std::str::FromStr for PortRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split('-').collect();
        if parts.len() != 2 {
            bail!("expected start-end");
        }

        let parsed: Vec<u16> = parts
            .iter()
            .map(|p| p.parse::<u16>().expect("can't parse u16"))
            .collect();

        Ok(parsed.into())
    }
}
