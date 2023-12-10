use anyhow::Result;
use std::net::IpAddr;
use std::{collections::HashSet, sync::Arc};
use structopt::StructOpt;
use tokio::{
    sync::{mpsc, Semaphore},
    time::{sleep, Duration},
};
use tracing::{debug, info, instrument};
use tsunami::{
    cli::{Opt, PortRange},
    net::{get_default_gateway_interface, to_ipaddr},
    receiver::receive,
    worker::inspect,
    {Message, Port},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opt::from_args();

    if opts.ports.is_none() && opts.ranges.is_none() {
        eprintln!("either port(s) or range(s) required");
        return;
    }

    if let Err(e) = run(
        &opts.target,
        &opts.ports.unwrap_or_default(),
        &opts.ranges.unwrap_or_default(),
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

#[instrument(skip_all, name = "main thread")]
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

    debug!("obtained default gateway interface ip: {:?}", ip_addr);

    let combined: HashSet<_> = ports
        .iter()
        .copied()
        .chain(ranges.iter().flat_map(|r| (r.start..=r.end)))
        .collect();

    info!(
        "initiating inspection for {} ({} ports) - mr: {} - bs: {} - nas: {} - nab: {}",
        target,
        combined.len(),
        max_retries,
        batch_size,
        nap_after_spawn,
        nap_after_batch,
    );

    /* receiver2mainthread */
    let (tx, mut rx) = mpsc::channel(8);

    debug!("spawning receiver");
    let receiver = tokio::spawn(receive(combined, tx, max_retries));

    /* This semaphore controls the maximum number of tasks in flight. */
    let semaphore = Arc::new(Semaphore::new(flying_tasks as usize));

    /* The main thread awaits messages from the receiver.
     * A message contains a Vec<Port> payload that tells the main thread
     * which ports to inspect. The ports are not dispatched immediately,
     * but are first sliced into 'batch_size' sized chunks (for rate limiting). */
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Payload(payload) => {
                debug!("got payload of size {}", payload.len());

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

                    /* Sleep a little after the sent batch, for good measure. */
                    sleep(Duration::from_secs_f64(nap_after_batch / 1000.0)).await;
                }

                debug!("dispatched the entire payload of size {}", payload.len());
            }
            Message::Break => {
                info!("got Message::break, breaking");
                break;
            }
        }
    }

    receiver.await??;
    debug!("awaited receiver");

    info!("exiting");
    Ok(())
}
