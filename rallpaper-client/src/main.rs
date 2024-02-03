use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
    let mut conn = UnixStream::connect("/tmp/rallpaper.sock").unwrap();
    write!(conn, "close").unwrap();
    conn.shutdown(std::net::Shutdown::Write).unwrap();
    conn.read_to_end(&mut Vec::new()).unwrap();
}
