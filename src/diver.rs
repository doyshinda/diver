use super::AppConfig;
use log::debug;
use std::{
    io::{Error, ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::{sleep, spawn},
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

fn downstream_receiver(
    mut downstream: TcpStream,
    mut upstream: TcpStream,
    buf_len: usize,
    drop: bool,
) {
    downstream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut buf = Vec::with_capacity(buf_len);
    reset_buf(&mut buf, buf_len);

    loop {
        let last_read = handle_read(&mut downstream, &mut buf);

        if last_read > 0 {
            if !drop {
                handle_write(&mut upstream, &mut buf[..last_read]);
            }
            reset_buf(&mut buf, buf_len);
        } else {
            break;
        }
    }
}

fn handle_connection(
    mut upstream: TcpStream,
    addr: &str,
    drop_addr: &str,
    num_conn: Arc<AtomicUsize>,
    buf_len: usize,
) {
    num_conn.fetch_add(1, Ordering::Relaxed);
    let mut downstream = TcpStream::connect(addr).unwrap();
    let d_clone = downstream.try_clone().unwrap();
    let u_clone = upstream.try_clone().unwrap();
    spawn(move || downstream_receiver(d_clone, u_clone, buf_len, false));

    let mut downstream2 = TcpStream::connect(drop_addr).unwrap();
    let d2_clone = downstream2.try_clone().unwrap();
    let u2_clone = upstream.try_clone().unwrap();
    spawn(move || downstream_receiver(d2_clone, u2_clone, buf_len, true));

    let mut buf = Vec::with_capacity(buf_len);
    reset_buf(&mut buf, buf_len);

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
            reset_buf(&mut buf, buf_len);
        } else {
            break;
        }
    }
    num_conn.fetch_sub(1, Ordering::Relaxed);
}

pub fn run(cfg: AppConfig) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", &cfg.port)).unwrap();
    let num_conn = Arc::new(AtomicUsize::new(0));
    let max_conn = cfg.max_conn.unwrap_or(1000);
    let buf_size = cfg.buffer_size_bytes;
    loop {
        if num_conn.load(Ordering::Relaxed) < max_conn {
            let accepted = listener.accept();
            match accepted {
                Ok((s, a)) => {
                    debug!("new connection from: {}", string_addr(Ok(a)));
                    let real = cfg.real.clone();
                    let test = cfg.test.clone();
                    let n_conn = num_conn.clone();
                    spawn(move || handle_connection(s, &real, &test, n_conn, buf_size));
                }
                Err(e) => println!("no stream: {:?}", e),
            }
        } else {
            sleep(Duration::from_millis(5));
        }
    }
}
