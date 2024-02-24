use color_eyre::eyre::Result;
use listener::Listener;
use tokio::sync::mpsc;
use worker::Worker;

mod listener;
mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    libnotify::init("cadence").unwrap();
    // note that we implicitly are assuming two daemons will never run concurrently here
    std::fs::remove_file("/tmp/rallpaper.sock")
        .unwrap_or_else(|_| println!("problem destructing socket file"));
    let (tx_worker, rx_worker) = mpsc::channel(10);

    tokio::spawn(Listener::new(tx_worker).unwrap().run());

    Worker::new(rx_worker).run().await;

    Ok(())
}
