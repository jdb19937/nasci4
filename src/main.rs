use std::{
    env,
    fs, fs::File,
    io::{self, prelude::*, BufRead, BufReader, ErrorKind, Lines},
    net::{TcpListener, TcpStream, UdpSocket},
    thread, path, path::Path,
    time::Duration,
};
use threadpool::ThreadPool;

fn handle(mut c : TcpStream) {
    let buf_reader = BufReader::new(&mut c);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("hello.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    c.write_all(response.as_bytes()).unwrap();
}

fn userver(us : UdpSocket) {
    let mut buf = [0u8; 1024];

    loop {
        let (result, src_addr) = us.recv_from(&mut buf).expect("recv failed");
        println!("I received {} bytes!", result);
        us.send_to(b"hello", src_addr).expect("send_to failed");
    }
}

fn heartbeat(us : UdpSocket, peers : Vec<String>) {
    let mut buf = [0u8; 1024];

    for peer in &peers {
      println!("peer={}", peer);
    }

    loop {
        println!("beat");
        thread::sleep(Duration::from_secs(5));
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let hostport = &args[1];

    let mut peers :Vec<String> = Vec::new();
    let mut foundself = 0;

    for peer in std::fs::read_to_string("peers.txt") .expect("file not found!").lines() {
      if &peer.to_string() == hostport {
        foundself = 1;
      } else {
        peers.push(peer.to_string());
      }
    }

    if foundself == 0 {
      println!("self not found in peers.txt");
      return;
    }

    let us0 = UdpSocket::bind(hostport).expect("bind failed");
    let us1 = us0.try_clone().expect("clone failed");
    thread::spawn(|| { userver(us0); });
    thread::spawn(|| { heartbeat(us1, peers); });

    let s = TcpListener::bind("127.0.0.1:7878").unwrap();

    let pool = ThreadPool::new(8);
    for c in s.incoming() {
        let c = c.unwrap();
	pool.execute(|| { handle(c); });
    }
}
