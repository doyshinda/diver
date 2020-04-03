use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread::spawn;
use std::time::Duration;

fn downstream_receiver_drop(mut downstream: TcpStream) {
    downstream.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
    let mut buf = Vec::with_capacity(2048);
    buf.resize_with(2048, || 0);
    let mut no_read_count = 0;
    while no_read_count < 100 {
        let last_read = match downstream.read(&mut buf) {
            Ok(n) => n,
            Err(e) => panic!("drop read error: {:?}", e),
        };

        if last_read == 0 {
            no_read_count += 1;
        }
        buf.clear();
    }
}

fn downstream_receiver(mut downstream: TcpStream, mut upstream: TcpStream) {
    downstream.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
    let mut buf = Vec::with_capacity(2048);
    buf.resize_with(2048, || 0);
    let mut no_read_count = 0;
    while no_read_count < 100 {
        let last_read = match downstream.read(&mut buf) {
            Ok(n) => n,
            Err(e) => panic!("read error: {:?}", e),
        };

        if last_read > 0 {
            no_read_count = 0;
            match upstream.write(&mut buf[..last_read]) {
                Ok(_) => (),
                Err(e) => panic!("write error: {:?}", e),
            }
        } else {
            no_read_count += 1;
        }
        buf.clear();
    }
}

fn handle_connection(mut stream: TcpStream) {
    stream.set_read_timeout(Some(Duration::from_millis(100))).unwrap();

    let mut downstream = TcpStream::connect("localhost:6080").unwrap();
    let d_clone = downstream.try_clone().unwrap();
    let s_clone = stream.try_clone().unwrap();
    spawn(move || downstream_receiver(d_clone, s_clone));

    let mut downstream2 = TcpStream::connect("localhost:6081").unwrap();
    let d2_clone = downstream2.try_clone().unwrap();
    spawn(move || downstream_receiver_drop(d2_clone));

    let mut buf = Vec::with_capacity(2048);
    buf.resize_with(2048, || 0);
    let mut no_read_count = 0;
    while no_read_count < 100 {
        let last_read = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(e) => panic!("handle_connection err: {:?}", e),
        };

        if last_read > 0 {
            match downstream.write(&mut buf[..last_read]) {
                Ok(_) => (),
                Err(e) => panic!("downstream write err: {:?}", e),
            }

            match downstream2.write(&mut buf[..last_read]) {
                Ok(_) => (),
                Err(e) => panic!("downstream2 write err: {:?}", e),
            }
        } else {
            no_read_count += 1;
        }
        buf.clear();
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("localhost:8008").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                spawn(move || handle_connection(s));
            }
            Err(e) => println!("no stream: {:?}", e),
        }
    }
    Ok(())
}
