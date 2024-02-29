use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use color_eyre::eyre::{self, eyre, Context, Result};
use listener::Listener;
use tokio::sync::mpsc;
use worker::Worker;

mod listener;
mod worker;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    Daemon,
    Client {
        #[command(subcommand)]
        client_command: ClientCommand,
    },
}

#[derive(Subcommand, Clone)]
enum ClientCommand {
    Toggle,
    Pomo,
    SkipWallpaper,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Daemon => daemon_main(),
        Command::Client { client_command } => client_main(client_command),
    }
}

fn client_main(command: ClientCommand) -> Result<()> {
    let mut conn =
        UnixStream::connect("/tmp/rallpaper.sock").wrap_err("client cannot connect to daemon")?;
    let message = match command {
        ClientCommand::Toggle => "toggle",
        ClientCommand::Pomo => "pomo",
        ClientCommand::SkipWallpaper => "change",
    };
    write!(conn, "{}", message).wrap_err("client failed to write to daemon socket")?;
    conn.shutdown(std::net::Shutdown::Write)
        .wrap_err("client failed to shutdown connection")?;
    let mut response = String::new();
    conn.read_to_string(&mut response)
        .wrap_err("client failed to receive response from daemon")?;
    println!("received: {}", response);
    Ok(())
}

#[tokio::main]
async fn daemon_main() -> Result<()> {
    if let Err(s) = libnotify::init("cadence") {
        return Err(eyre!("failed to initialise libnotify: {}", s));
    }
    // note that we implicitly are assuming two daemons will never run concurrently here
    std::fs::remove_file("/tmp/rallpaper.sock")
        .unwrap_or_else(|_| println!("problem destructing socket file"));
    let (tx_worker, rx_worker) = mpsc::channel(10);

    tokio::spawn(Listener::new(tx_worker).unwrap().run());

    Worker::new(rx_worker).run().await;
    Ok(())
}
