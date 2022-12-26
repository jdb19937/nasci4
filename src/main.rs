use std::{
    env,
    fs,
    io::{prelude::*, BufRead, BufReader},
    net::{TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use threadpool::ThreadPool;


fn handle(mut c : TcpStream, indx : Arc<RwLock<BTreeMap<u64,u64>>>, us : UdpSocket, peers : Vec<String>) {
    let buf_reader = BufReader::new(&mut c);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // println!("Request: {:#?}", http_request);

    let rparts : Vec<&str> = http_request[0].split(" ").collect::<Vec<&str>>();
    if rparts[0] != "GET" && rparts[0] != "POST" {
      return;
    }
    let url = rparts[1];
    if &url[0..1] != "/" {
      return;
    }

    let mut k : u64 = 0;
    let mut v : u64 = 0;
    let mut op = "get";

    if url.len() > 2 && &url[1..2] == "?" {
      let qs = &url[2..];
      let args = qs.split("&").collect::<Vec<&str>>();
      for arg in args {
        let kv = arg.split("=").collect::<Vec<&str>>();
        let qk = kv[0].to_string();
        let qv = kv[1].to_string();
        if qk == "k" && qv.len() > 0 {
          k = qv.parse::<u64>().expect("int");
        }
        if qk == "v" && qv.len() > 0 {
          v = qv.parse::<u64>().expect("int");
        }
        if qk == "op" && qv == "set" {
          op = "set";
        }
      }
    }

    let mut ind = indx.write().expect("can't get index");
    if op == "get" {
      match ind.get(&k) {
        Some(vopt) => { v = *vopt; }
        None => { v = 0; }
      }
    }

    if op == "set" {
        ind.insert(k, v);
        for peer in &peers {
            let msg0 = u64::to_be_bytes(k);
            let msg1 = u64::to_be_bytes(v);
            let mut msg : [u8; 16] = [0u8; 16];
            msg[0..8].copy_from_slice(&msg0);
            msg[8..16].copy_from_slice(&msg1);
            us.send_to(&msg, peer).expect("send fail");
        }
    }

    let status = "HTTP/1.1 200 OK";
    let mut contents = fs::read_to_string("index.html").unwrap();
    contents = contents.replace("{k}", &k.to_string());
    contents = contents.replace("{v}", &v.to_string());
    let length = contents.len();

    let response = format!("{status}\r\nContent-Length: {length}\r\n\r\n{contents}");

    c.write_all(response.as_bytes()).unwrap();

}

fn userver(us : UdpSocket, indx : Arc<RwLock<BTreeMap::<u64,u64>>>, _peers : Vec<String>) {
    let mut buf = [0u8; 1024];

    loop {
        for elem in buf.iter_mut() { *elem = 0; }
        let (result, _src_addr) = us.recv_from(&mut buf).expect("recv failed");
        if result != 16 {
            continue;
        }
        
        let k = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        let v = u64::from_be_bytes(buf[8..16].try_into().unwrap());
        // println!("I received {}={} from {}!", k,v, src_addr);
        // us.send_to(b"hello", src_addr).expect("send_to failed");

        let mut ind = indx.write().expect("can't get index");
        ind.insert(k, v);
//        if let Some(vp) = ind.get(&k) {
//            let mut wp = vp.lock().expect("can't get element");
//            *wp = v;
//        }
    }
}

fn heartbeat(us : UdpSocket, peers : Vec<String>) {
    // let buf = [0u8; 1024];

    loop {
        // println!("beat");
        thread::sleep(Duration::from_secs(5));
        for peer in &peers {
            us.send_to(b"hello", peer).expect("send fail");
        }
    }
}


fn main() {
    let ind = Arc::new(RwLock::new(BTreeMap::<u64,u64>::new()));

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

    let us = UdpSocket::bind(hostport).expect(&format!("bind failed { }", hostport));
    let us0 = us.try_clone().expect("clone failed");
    let ind1 = Arc::clone(&ind);
    let peers1 = peers.clone();
    thread::spawn(|| { userver(us0, ind1, peers1); });
    let us1 = us.try_clone().expect("clone failed");
    let peers2 = peers.clone();
    thread::spawn(|| { heartbeat(us1, peers2); });

    let s = TcpListener::bind(hostport).unwrap();

    let pool = ThreadPool::new(8);
    for c in s.incoming() {
        let c = c.unwrap();
        let ind2 = Arc::clone(&ind);
        let peers3 = peers.clone();
        let us3 = us.try_clone().expect("clone failed");
	pool.execute(|| { handle(c, ind2, us3, peers3); });
    }
}
