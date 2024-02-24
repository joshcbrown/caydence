use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const HELP_MESSAGE: &str = "usage: rallpaper-client [change|toggle|time]";

fn main() {
    let command: String = env::args().nth(1).expect(HELP_MESSAGE);
    if ["change", "toggle", "time"]
        .into_iter()
        .all(|accepted_command| accepted_command != &command)
    {
        println!("{}", HELP_MESSAGE);
        std::process::exit(1)
    }

    let mut conn = UnixStream::connect("/tmp/rallpaper.sock").unwrap();
    write!(conn, "{}", command).unwrap();
    conn.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    conn.read_to_string(&mut response).unwrap();
    println!("received: {}", response);
}
