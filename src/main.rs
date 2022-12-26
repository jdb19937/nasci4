use std::{
    env,
    fs,
    io::{prelude::*, BufRead, BufReader},
    net::{TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};
use threadpool::ThreadPool;

mod store;

fn handle(mut c : TcpStream, ind : Arc<RwLock<BTreeMap<u64,u64>>>) {
    let buf_reader = BufReader::new(&mut c);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    let status = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("hello.html").unwrap();
    let length = contents.len();

    let response = format!("{status}\r\nContent-Length: {length}\r\n\r\n{contents}");

    c.write_all(response.as_bytes()).unwrap();
}

fn userver(us : UdpSocket, indx : Arc<RwLock<BTreeMap::<u64,u64>>>) {
    let mut buf = [0u8; 1024];

    loop {
        for elem in buf.iter_mut() { *elem = 0; }
        let (result, src_addr) = us.recv_from(&mut buf).expect("recv failed");
        if result != 16 {
            continue;
        }
        
        let mut k = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        let mut v = u64::from_be_bytes(buf[8..16].try_into().unwrap());
        println!("I received {}={} from {}!", k,v, src_addr);
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
    let mut buf = [0u8; 1024];

    loop {
        println!("beat");
        thread::sleep(Duration::from_secs(5));
        for peer in &peers {
            us.send_to(b"hello", peer);
        }
    }
}


fn main() {
    let mut ind = Arc::new(RwLock::new(BTreeMap::<u64,u64>::new()));

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

    let us0 = UdpSocket::bind(hostport).expect(&format!("bind failed { }", hostport));
    let us1 = us0.try_clone().expect("clone failed");
    let ind1 = Arc::clone(&ind);
    thread::spawn(|| { userver(us0, ind1); });
    thread::spawn(|| { heartbeat(us1, peers); });

    let s = TcpListener::bind(hostport).unwrap();

    let pool = ThreadPool::new(8);
    for c in s.incoming() {
        let c = c.unwrap();
        let ind2 = Arc::clone(&ind);
	pool.execute(|| { handle(c, ind2); });
    }
}
