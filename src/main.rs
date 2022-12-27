use std::{
    env,
    fs,
    io::{prelude::*, BufRead, BufReader},
    net::{TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
    sync::{Arc, RwLock},
};
use threadpool::ThreadPool;

mod hashtree;

use crate::hashtree::HashTree;

fn handle(mut c : TcpStream, indx : Arc<RwLock<HashTree>>, us : UdpSocket, peers : Vec<String>, logfn : String) {
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
      if ind.lookup(k) {
        v = 1;
      } else {
        v = 0;
      }
    }

    if op == "set" {
        ind.insert(k);
        v = 1;
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

    let logstr = fs::read_to_string(logfn).unwrap();
    contents.push_str("<hr><pre>");
    contents.push_str(&logstr);
    contents.push_str("</pre></body></html>");

    let length = contents.len();

    let response = format!("{status}\r\nContent-Length: {length}\r\n\r\n{contents}");

    c.write_all(response.as_bytes()).unwrap();

}

fn send_expand_pkt(s : &UdpSocket, addr : String, pre : u64, h : u64, hl : u64, hr : u64) {
    println!("sending expand packet addr={addr} pre={pre} h={h} hl={hl} hr={hr}");

    let tgt = u64::to_be_bytes(pre);
    let root = u64::to_be_bytes(h);
    let left = u64::to_be_bytes(hl);
    let right = u64::to_be_bytes(hr);

    let mut msg : [u8; 40] = [0u8; 40];
    let rop = u64::to_be_bytes(37);
    msg[0..8].copy_from_slice(&rop);
    msg[8..16].copy_from_slice(&tgt);
    msg[16..24].copy_from_slice(&root);
    msg[24..32].copy_from_slice(&left);
    msg[32..40].copy_from_slice(&right);

    s.send_to(&msg, addr).expect("send fail");
}

fn send_request_pkt(s : &UdpSocket, addr : String, pre : u64) {
    println!("sending request packet addr={addr} pre={pre}");

    let tgt = u64::to_be_bytes(pre);

    let mut msg : [u8; 16] = [0u8; 16];
    let rop = u64::to_be_bytes(38);
    msg[0..8].copy_from_slice(&rop);
    msg[8..16].copy_from_slice(&tgt);

    s.send_to(&msg, addr).expect("send fail");
}

fn send_key_pkt(s : &UdpSocket, addr : String, key : u64) {
    println!("sending key packet addr={addr} key={key}");

    let tgt = u64::to_be_bytes(key);

    let mut msg : [u8; 16] = [0u8; 16];
    let rop = u64::to_be_bytes(39);
    msg[0..8].copy_from_slice(&rop);
    msg[8..16].copy_from_slice(&tgt);

    s.send_to(&msg, addr).expect("send fail");
}




fn userver(us : UdpSocket, indx : Arc<RwLock<HashTree>>, _peers : Vec<String>) {
    let mut buf = [0u8; 1024];

    loop {
        for elem in buf.iter_mut() { *elem = 0; }
        let (pktlen, src_addr) = us.recv_from(&mut buf).expect("recv failed");
        if pktlen < 16 {
            continue;
        }
        
        let op = u64::from_be_bytes(buf[0..8].try_into().unwrap());

        if op == 37 {
            if pktlen != 40 {
                continue;
            }

            let pre = u64::from_be_bytes(buf[8..16].try_into().unwrap());
            let prel = pre * 2;
            let prer = pre * 2 + 1;
            let h = u64::from_be_bytes(buf[16..24].try_into().unwrap());
            let hl = u64::from_be_bytes(buf[24..32].try_into().unwrap());
            let hr = u64::from_be_bytes(buf[32..40].try_into().unwrap());

            println!("received expand packet addr={src_addr} pre={pre} h={h} hl={hl} hr={hr}");

            let ind = indx.read().expect("can't get index");

            if ind.prehash(pre) != h {
                if hl != 0 && ind.prehash(prel) != hl {
                    send_request_pkt(&us, src_addr.to_string(), prel);
                }
                if hr != 0 && ind.prehash(prer) != hr {
                    send_request_pkt(&us, src_addr.to_string(), prer);
                }
            }
        }

        if op == 38 {
            let ind = indx.read().expect("can't get index");

            if pktlen != 16 {
                continue;
            }

            let pre = u64::from_be_bytes(buf[8..16].try_into().unwrap());

            println!("received request packet addr={src_addr} pre={pre}");

            if ind.precount(pre) == 1 {
                send_key_pkt(&us, src_addr.to_string(), ind.hashkey(ind.prehash(pre)));
            } else if ind.precount(pre) > 1 {
                send_expand_pkt(&us, src_addr.to_string(), pre, ind.prehash(pre), ind.prehash(pre * 2), ind.prehash(pre * 2 + 1));
            }
        }

        if op == 39 {
            if pktlen != 16 {
                continue;
            }

            let key = u64::from_be_bytes(buf[8..16].try_into().unwrap());

            println!("received key packet addr={src_addr} key={key}");
            let mut ind = indx.write().expect("can't get index");
            ind.insert(key);
        }
    }
}

fn heartbeat(us : UdpSocket, indx : Arc<RwLock<HashTree>>, peers : Vec<String>) {
    // let buf = [0u8; 1024];
    let duration = 10;

    loop {
        thread::sleep(Duration::from_secs(duration));
        // println!("starting heartbeats");

        let ind = indx.read().expect("can't get index");

        for peer in &peers {
            // println!("running heartbeat for addr={peer}");
            send_expand_pkt(&us, peer.to_string(), 1, ind.prehash(1), ind.prehash(2), ind.prehash(3));
            // println!("done with heartbeat for addr={peer}");
        }
        // println!("done with heartbeats, sleeping for {duration} seconds");
    }
}


fn main() {
    let ind = Arc::new(RwLock::new(HashTree::new()));

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
      // println!("self not found in peers.txt");
      return;
    }

    let us = UdpSocket::bind(hostport).expect(&format!("bind failed { }", hostport));
    let us0 = us.try_clone().expect("clone failed");
    let ind1 = Arc::clone(&ind);
    let peers1 = peers.clone();
    thread::spawn(|| { userver(us0, ind1, peers1); });
    let us1 = us.try_clone().expect("clone failed");
    let peers2 = peers.clone();
    let ind2 = Arc::clone(&ind);
    thread::spawn(|| { heartbeat(us1, ind2, peers2); });

    let s = TcpListener::bind(hostport).unwrap();

    let pool = ThreadPool::new(8);
    for c in s.incoming() {
        let c = c.unwrap();
        let ind2 = Arc::clone(&ind);
        let peers3 = peers.clone();
        let us3 = us.try_clone().expect("clone failed");
        let mut logfn = String::new();
        logfn.push_str("log.");
        logfn.push_str(hostport);
        logfn.push_str(".txt");
	pool.execute(|| { handle(c, ind2, us3, peers3, logfn); });
    }
}
