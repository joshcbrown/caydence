use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use color_eyre::eyre::{eyre, Context, Result};
use listener::Listener;
use tokio::sync::mpsc;
use worker::Worker;

mod listener;
mod worker;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// initialise daemon. see `help daemon` for arguments.
    Daemon(DaemonArgs),
    /// pass a message to the daemon. see `help client` for possible values.
    Client {
        #[command(subcommand)]
        client_command: ClientCommand,
    },
}

#[derive(Args)]
struct DaemonArgs {
    /// path to directory with wallpapers.
    wallpaper_dir: PathBuf,
    /// framerate for wallpaper transition effect.
    #[arg(short, long, default_value = "140")]
    transition_fps: u8,
    #[arg(short, long, default_value = "20")]
    regular_interval_mins: u8,
    #[arg(long, default_value = "25")]
    work_mins: u8,
    #[arg(long, default_value = "5")]
    short_break_mins: u8,
    #[arg(long, default_value = "15")]
    long_break_mins: u8,
    #[arg(long, default_value = "4")]
    cycles_before_break: u8,
}

#[derive(Subcommand, Clone)]
enum ClientCommand {
    /// switch between pomodoro cycles and 20-min intervals.
    Toggle,
    /// query the daemon for time remaining in current cycle.
    Time,
    /// skip current cycle, including the currently running pomo
    /// if applicable.
    Skip,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    match cli.command {
        Command::Daemon(args) => daemon_main(args),
        Command::Client { client_command } => client_main(client_command),
    }
}

fn client_main(command: ClientCommand) -> Result<()> {
    let mut conn =
        UnixStream::connect("/tmp/rallpaper.sock").wrap_err("client cannot connect to daemon")?;
    let message = match command {
        ClientCommand::Toggle => "toggle",
        ClientCommand::Time => "time",
        ClientCommand::Skip => "skip",
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
async fn daemon_main(args: DaemonArgs) -> Result<()> {
    if let Err(s) = libnotify::init("cadence") {
        return Err(eyre!("failed to initialise libnotify: {}", s));
    }
    if !args.wallpaper_dir.is_dir() {
        return Err(eyre!("{:?} is not a directory", args.wallpaper_dir));
    }
    // FIX: check for daemon already running
    std::fs::remove_file("/tmp/rallpaper.sock")
        .unwrap_or_else(|_| println!("problem destructing socket file"));
    let (tx_worker, rx_worker) = mpsc::channel(10);

    tokio::spawn(Listener::new(tx_worker).unwrap().run());

    Worker::new(rx_worker, args).run().await;
    Ok(())
}
