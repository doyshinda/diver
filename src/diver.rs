use super::AppConfig;
use log::debug;
use std::{
    io::{Error, ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::spawn,
    time::Duration,
};

#[inline]
fn reset_buf(buf: &mut Vec<u8>, buf_len: usize) {
    buf.resize(buf_len, 0);
}

fn string_addr(addr: Result<SocketAddr, Error>) -> String {
    match addr {
        Ok(ref a) => format!("{}:{}", a.ip(), a.port()),
        Err(e) => format!("unknown: {:?}", e),
    }
}

fn handle_read(stream: &mut TcpStream, buf: &mut Vec<u8>) -> usize {
    match stream.read(buf) {
        Ok(n) => {
            if n > 0 {
                debug!(
                    "[{:>21}] <--- [{:<21}] recv {} bytes",
                    string_addr(stream.local_addr()),
                    string_addr(stream.peer_addr()),
                    n,
                );
            }
            n
        }
        Err(e) => {
            if e.kind() == ErrorKind::WouldBlock {
                0
            } else {
                panic!("read error: {:?}", e);
            }
        }
    }
}

fn handle_write(stream: &mut TcpStream, buf: &mut [u8]) {
    match stream.write(buf) {
        Ok(_) => {
            debug!(
                "[{:>21}] ---> [{:<21}] sent {} bytes",
                string_addr(stream.local_addr()),
                string_addr(stream.peer_addr()),
                buf.len(),
            );
        }
        Err(e) => panic!("write error: {:?}", e),
    };
}

fn downstream_receiver(mut downstream: TcpStream, mut upstream: TcpStream, drop: bool) {
    downstream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let buflen: usize = 1000;
    let mut buf = Vec::with_capacity(buflen);
    reset_buf(&mut buf, buflen);

    loop {
        let last_read = handle_read(&mut downstream, &mut buf);

        if last_read > 0 {
            if !drop {
                handle_write(&mut upstream, &mut buf[..last_read]);
            }
            reset_buf(&mut buf, buflen);
        } else {
            break;
        }
    }
}

fn handle_connection(mut upstream: TcpStream, addr: &str, drop_addr: &str) {
    let mut downstream = TcpStream::connect(addr).unwrap();
    let d_clone = downstream.try_clone().unwrap();
    let u_clone = upstream.try_clone().unwrap();
    spawn(move || downstream_receiver(d_clone, u_clone, false));

    let mut downstream2 = TcpStream::connect(drop_addr).unwrap();
    let d2_clone = downstream2.try_clone().unwrap();
    let u2_clone = upstream.try_clone().unwrap();
    spawn(move || downstream_receiver(d2_clone, u2_clone, true));

    let buflen: usize = 1000;
    let mut buf = Vec::with_capacity(buflen);
    reset_buf(&mut buf, buflen);

    upstream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    downstream
        .set_write_timeout(Some(Duration::from_millis(10)))
        .unwrap();
    downstream2
        .set_write_timeout(Some(Duration::from_millis(10)))
        .unwrap();

    loop {
        let bytes_recv = handle_read(&mut upstream, &mut buf);

        if bytes_recv > 0 {
            handle_write(&mut downstream, &mut buf[..bytes_recv]);
            handle_write(&mut downstream2, &mut buf[..bytes_recv]);
            reset_buf(&mut buf, buflen);
        } else {
            break;
        }
    }
}

pub fn run(cfg: AppConfig) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", &cfg.port)).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                debug!("new connection from: {}", string_addr(s.peer_addr()));
                let real = cfg.real.clone();
                let test = cfg.test.clone();
                spawn(move || handle_connection(s, &real, &test));
            }
            Err(e) => println!("no stream: {:?}", e),
        }
    }
}
