use anyhow::Result;
use std::net::IpAddr;
use std::{collections::HashSet, sync::Arc};
use structopt::StructOpt;
use tokio::{
    sync::{mpsc, Semaphore},
    time::{sleep, Duration},
};
use tsunami::{
    cli::{Opt, PortRange},
    net::{get_default_gateway_interface, to_ipaddr},
    receiver::receive,
    worker::inspect,
    {Message, Port},
};

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(
        &opts.target,
        &opts.ports,
        &opts.ranges,
        opts.flying_tasks,
        opts.max_retries,
        opts.batch_size,
        opts.nap_after_spawn,
        opts.nap_after_batch,
    )
    .await
    {
        eprintln!("tsunami: {:?}", e);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run(
    target: &str,
    ports: &[Port],
    ranges: &[PortRange],
    flying_tasks: u16,
    max_retries: usize,
    batch_size: usize,
    nap_after_spawn: f64,
    nap_after_batch: f64,
) -> Result<()> {
    let ip_addr = match get_default_gateway_interface()? {
        IpAddr::V4(ipv4) => ipv4,
        _ => unimplemented!(),
    };

    let combined: HashSet<_> = ports
        .iter()
        .copied()
        .chain(ranges.iter().flat_map(|r| (r.start..=r.end)))
        .collect();

    let (tx, mut rx) = mpsc::channel(8);

    let receiver = tokio::spawn(receive(combined, tx, max_retries));

    let semaphore = Arc::new(Semaphore::new(flying_tasks as usize));

    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Payload(payload) => {
                for chunk in payload.chunks(batch_size) {
                    let mut tasks = vec![];

                    for port in chunk {
                        tasks.push(tokio::spawn(inspect(
                            to_ipaddr(target).await?,
                            *port,
                            semaphore.clone(),
                            ip_addr,
                            nap_after_spawn,
                        )));
                    }

                    for task in tasks {
                        task.await??;
                    }

                    sleep(Duration::from_secs_f64(nap_after_batch / 1000.0)).await;
                }
            }
            Message::Break => break,
        }
    }

    receiver.await??;

    Ok(())
}
