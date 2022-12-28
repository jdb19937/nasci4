use rand::Rng;
use std::{
    env, fs,
    io::{prelude::*, BufRead, BufReader},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
    time::SystemTime,
    time::UNIX_EPOCH,
};
use threadpool::ThreadPool;

mod hashtree;

use crate::hashtree::HashTree;
use crate::hashtree::ValueProof;

fn handle(mut c: TcpStream, indx: Arc<RwLock<HashTree>>, logfn: String) {
    let buf_reader = BufReader::new(&mut c);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // println!("Request: {:#?}", http_request);

    if http_request.len() < 1 {
        return;
    }

    let rparts: Vec<&str> = http_request[0].split(" ").collect::<Vec<&str>>();
    if rparts.len() < 1 || rparts[0] != "GET" && rparts[0] != "POST" {
        return;
    }
    let url = rparts[1];
    if &url[0..1] != "/" {
        return;
    }

    let mut k: u64 = 0;
    let mut v: u64 = 0;
    let mut ts: u64 = 0;
    let mut logwork: f64 = 0.0;
    let mut seed: u64 = 0;
    let mut h: u64 = 0;
    let mut op = "get";
    let mut minlogwork: f64 = -8.0;

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
            if qk == "minlogwork" {
                minlogwork = qv.parse::<f64>().expect("float");
            }
        }
    }

    let mut ind = indx.write().expect("can't get index");

    if op == "set" {
        let mut rng = rand::thread_rng();
        let mut vp = ValueProof::new();

        vp.k = k;
        vp.v = v;
        vp.ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("AD")
            .as_secs();

        vp.seed = 0;
        vp.compute_hash();

        while vp.logwork() < minlogwork {
            vp.seed = rng.gen::<u64>();
            vp.compute_hash();
        }

        ind.insert(&vp);
    }

    if true {
        let ovp = ind.lookup(k);
        if !ovp.is_none() {
            let vp: &ValueProof = ovp.expect("found");
            v = vp.v;
            h = vp.h;
            seed = vp.seed;
            ts = vp.ts;
            logwork = vp.logwork();
        } else {
            v = 0;
        }
    }

    let status = "HTTP/1.1 200 OK";
    let mut contents = fs::read_to_string("index.html").unwrap();
    contents = contents.replace("{k}", &k.to_string());
    contents = contents.replace("{v}", &v.to_string());
    contents = contents.replace("{ts}", &ts.to_string());
    contents = contents.replace("{seed}", &seed.to_string());
    contents = contents.replace("{h}", &h.to_string());
    contents = contents.replace("{logwork}", &logwork.to_string());

    let logstr = fs::read_to_string(logfn).unwrap();
    contents.push_str("<hr><pre>");
    contents.push_str(&logstr);
    contents.push_str("</pre></body></html>");

    let length = contents.len();

    let response = format!("{status}\r\nContent-Length: {length}\r\n\r\n{contents}");

    c.write_all(response.as_bytes()).unwrap();
}

fn send_expand_pkt(s: &UdpSocket, addr: String, pre: u64, h: u64, hl: u64, hr: u64) {
    println!("sending expand packet addr={addr} pre={pre} h={h} hl={hl} hr={hr}");

    assert!(h == hl ^ hr);

    let tgt = u64::to_be_bytes(pre);
    let root = u64::to_be_bytes(h);
    let left = u64::to_be_bytes(hl);
    let right = u64::to_be_bytes(hr);

    let mut msg: [u8; 40] = [0u8; 40];
    let rop = u64::to_be_bytes(37);
    msg[0..8].copy_from_slice(&rop);
    msg[8..16].copy_from_slice(&tgt);
    msg[16..24].copy_from_slice(&root);
    msg[24..32].copy_from_slice(&left);
    msg[32..40].copy_from_slice(&right);

    s.send_to(&msg, addr).expect("send fail");
}

fn send_request_pkt(s: &UdpSocket, addr: String, pre: u64) {
    println!("sending request packet addr={addr} pre={pre}");

    let tgt = u64::to_be_bytes(pre);

    let mut msg: [u8; 16] = [0u8; 16];
    let rop = u64::to_be_bytes(38);
    msg[0..8].copy_from_slice(&rop);
    msg[8..16].copy_from_slice(&tgt);

    s.send_to(&msg, addr).expect("send fail");
}

fn send_key_pkt(s: &UdpSocket, addr: String, vp: &ValueProof) {
    let key = vp.k;

    println!("sending key packet addr={addr} key={key}");

    let mut msg: [u8; 40] = [0u8; 40];
    let rop = u64::to_be_bytes(39);
    msg[0..8].copy_from_slice(&rop);

    let mut tmp = u64::to_be_bytes(vp.k);
    msg[8..16].copy_from_slice(&tmp);
    tmp = u64::to_be_bytes(vp.v);
    msg[16..24].copy_from_slice(&tmp);
    tmp = u64::to_be_bytes(vp.ts);
    msg[24..32].copy_from_slice(&tmp);
    tmp = u64::to_be_bytes(vp.seed);
    msg[32..40].copy_from_slice(&tmp);

    s.send_to(&msg, addr).expect("send fail");
}

fn userver(us: UdpSocket, indx: Arc<RwLock<HashTree>>, _peers: Vec<String>) {
    let mut buf = [0u8; 1024];

    loop {
        for elem in buf.iter_mut() {
            *elem = 0;
        }
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
                } else {
                    println!("not sending reqest packet for hl={hl} prel={prel}");
                }
                if hr != 0 && ind.prehash(prer) != hr {
                    send_request_pkt(&us, src_addr.to_string(), prer);
                } else {
                    println!("not sending reqest packet for hr={hr} prer={prer}");
                }
            } else {
                println!("h={h} matches index");
            }
        }

        if op == 38 {
            let ind = indx.read().expect("can't get index");

            if pktlen != 16 {
                continue;
            }

            let pre = u64::from_be_bytes(buf[8..16].try_into().unwrap());

            println!("received request packet addr={src_addr} pre={pre}");

            let h = ind.prehash(pre);
            if h > 0 {
                let k = ind.hashkey(h);
                if k > 0 {
                    //let hl = ind.prehash(pre * 2);
                    //let hr = ind.prehash(pre * 2 + 1);
                    //assert!(hl == 0 || hr == 0);

                    let vp = ind.keyproof(k);
                    send_key_pkt(&us, src_addr.to_string(), vp);
                } else {
                    let hl = ind.prehash(pre * 2);
                    let hr = ind.prehash(pre * 2 + 1);

                    send_expand_pkt(&us, src_addr.to_string(), pre, h, hl, hr);
                }
            }
        }

        if op == 39 {
            if pktlen != 40 {
                continue;
            }

            let k = u64::from_be_bytes(buf[8..16].try_into().unwrap());
            let v = u64::from_be_bytes(buf[16..24].try_into().unwrap());
            let ts = u64::from_be_bytes(buf[24..32].try_into().unwrap());
            let seed = u64::from_be_bytes(buf[32..40].try_into().unwrap());

            println!("received key packet addr={src_addr} key={k} val={v} ts={ts} seed={seed}");

            let mut vp = ValueProof::new();
            vp.k = k;
            vp.v = v;
            vp.ts = ts;
            vp.seed = seed;
            vp.compute_hash();

            let mut ind = indx.write().expect("can't get index");

            ind.insert(&vp);
        }
    }
}

fn heartbeat(us: UdpSocket, indx: Arc<RwLock<HashTree>>, peers: Vec<String>) {
    let duration = 10;

    loop {
        thread::sleep(Duration::from_secs(duration));
        // println!("starting heartbeats");

        let ind = indx.read().expect("can't get index");

        for peer in &peers {
            println!("running heartbeat for addr={peer}");
            if ind.prehash(1) > 0 {
                send_expand_pkt(
                    &us,
                    peer.to_string(),
                    1,
                    ind.prehash(1),
                    ind.prehash(2),
                    ind.prehash(3),
                );
            }
            println!("done with heartbeat for addr={peer}");
        }
        // println!("done with heartbeats, sleeping for {duration} seconds");
    }
}

fn main() {
    let ind = Arc::new(RwLock::new(HashTree::new()));

    let args: Vec<String> = env::args().collect();
    let hostport = &args[1];

    let mut peers: Vec<String> = Vec::new();
    let mut foundself = 0;

    for peer in std::fs::read_to_string("peers.txt")
        .expect("file not found!")
        .lines()
    {
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
    thread::spawn(|| {
        userver(us0, ind1, peers1);
    });
    let us1 = us.try_clone().expect("clone failed");
    let peers2 = peers.clone();
    let ind2 = Arc::clone(&ind);
    thread::spawn(|| {
        heartbeat(us1, ind2, peers2);
    });

    let s = TcpListener::bind(hostport).unwrap();

    let pool = ThreadPool::new(8);
    for c in s.incoming() {
        let c = c.unwrap();
        let ind2 = Arc::clone(&ind);
        let mut logfn = String::new();
        logfn.push_str("log.");
        logfn.push_str(hostport);
        logfn.push_str(".txt");
        pool.execute(|| {
            handle(c, ind2, logfn);
        });
    }
}
