// use sha256::{digest, try_digest};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const DECAY : f64 = 0.0001;
const SLACK : u64 = 2;

pub fn hash(key: u64) -> u64 {
    let mkey = key % 4294967291;
    return ((mkey * mkey) % 4294967291) + 1;
}

#[derive(Copy, Clone)]
pub struct ValueProof {
    pub k: u64,
    pub v: u64,
    pub ts: u64,
    pub seed: u64,
    pub h: u64,
}

impl ValueProof {
    pub fn new() -> Self {
        return ValueProof {
            k: 0,
            v: 0,
            ts: 0,
            seed: 0,
            h: 0,
        };
    }

    pub fn is_past_time(&self) -> bool {
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("AD")
            .as_secs();
        return self.ts < now + SLACK;
    }
    pub fn age(&self) -> i64 {
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("AD")
            .as_secs();
        let dt = (now + SLACK - self.ts) as i64;
        return dt;
    }

    pub fn compute_hash(&mut self) {
        self.h = hash(self.k ^ hash(self.v ^ hash(self.ts ^ hash(self.seed))));
    }
    pub fn hash_is_valid(&self) -> bool {
        let newh = hash(self.k ^ hash(self.v ^ hash(self.ts ^ hash(self.seed))));
        return self.h == newh;
    }
    pub fn is_valid(&self) -> bool {
        self.is_past_time() && self.hash_is_valid()
    }

    pub fn logwork(&self) -> f64 {
        // log(1/h * exp(-DECAY * age))
        // -log(h) - DECAY * age

        let fh = self.h as f64;
        let lfh = f64::ln(fh);
        let top = 4.0 * ((1 << 30) as f64);
        let ltop = f64::ln(top);
        let fage = self.age() as f64;
        // println!("fh={fh} lfh={lfh} top={top} ltop={ltop} fage={fage}");
        return ltop - lfh - DECAY * fage;
    }

    pub fn worth_more(&self, other: &ValueProof) -> bool {
        // 1/h0 * exp(-DECAY * (now-t0)) > 1/h1 * exp(-DECAY * (now-t1))
        // h1 > h0 * exp(DECAY * (now-t0) -DECAY * (now-t1))
        // h1 > h0 * exp(DECAY * ((now-t0)-(now-t1))
        // h1 > h0 * exp(DECAY * (t1 - t0))
        // h0 * exp(DECAY * (t1 - t0)) < h1
        // log(h0) + DECAY * dt < log(h1)

        let fh0 = self.h as f64;
        let flh0 = f64::ln(fh0);
        let fh1 = other.h as f64;
        let flh1 = f64::ln(fh1);

        let ft0 = self.ts as f64;
        let ft1 = other.ts as f64;

        let dt = ft1 - ft0;
        return flh0 + DECAY * dt < flh1;
    }
}

//impl Ord for ValueProof {
//    fn cmp(&self, other: &Self) -> Ordering {
//        if self.worth_more(other) {
//             Ordering::Greater
//        } else {
//             Ordering::Less
//        }
//    }
//}

impl PartialOrd for ValueProof {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.k != other.k {
            None
        } else if self.worth_more(other) {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}
impl PartialEq for ValueProof {
    fn eq(&self, _other: &Self) -> bool {
        return false;
    }
}
impl Eq for ValueProof {}

pub struct HashTree {
    pub prefix_hash: BTreeMap<u64, u64>,
    pub hash_key: BTreeMap<u64, u64>,
    pub key_proof: BTreeMap<u64, ValueProof>,
}

impl HashTree {
    pub fn new() -> Self {
        let s = HashTree {
            prefix_hash: BTreeMap::<u64, u64>::new(),
            hash_key: BTreeMap::<u64, u64>::new(),
            key_proof: BTreeMap::<u64, ValueProof>::new(),
        };
        return s;
    }

    pub fn lookup(&self, key: u64) -> Option<&ValueProof> {
        if !self.key_proof.contains_key(&key) {
            return None;
        }
        let vp: &ValueProof = self.key_proof.get(&key).expect("key not present");
        return Some(vp);
    }

    pub fn prehash(&self, pre: u64) -> u64 {
        if self.prefix_hash.contains_key(&pre) {
            return *self.prefix_hash.get(&pre).expect("key not present");
        } else {
            return 0;
        }
    }

    pub fn hashkey(&self, h: u64) -> u64 {
        if self.hash_key.contains_key(&h) {
            return *self.hash_key.get(&h).expect("key not present");
        } else {
            return 0;
        }
    }

    pub fn keyproof(&self, key: u64) -> &ValueProof {
        //        if self.key_proof.contains_key(&key) {
        return self.key_proof.get(&key).expect("key not present");
        //        } else {
        //            return &ValueProof::new();
        //        }
    }

    pub fn remove(&mut self, key: u64) {
        assert!(self.key_proof.contains_key(&key));
        let vp: &ValueProof = self.key_proof.get(&key).expect("key not present");

        let h = vp.h;
        self.hash_key.remove(&vp.h);
        let k1 = vp.k; // ???
        self.key_proof.remove(&k1);

        for b in 0..32 {
            let hpre = h >> b;
            if hpre > 0 {
                let c = self.prehash(hpre) ^ h;
                *self.prefix_hash.entry(hpre).or_insert(0) = c;
            }
        }
    }

    pub fn insert(&mut self, vp: &ValueProof) {
        let key = vp.k;

        println!("inserting key={key}");

        if !vp.is_valid() {
            println!("NOT VALID key={key}");
            return;
        }

        if self.key_proof.contains_key(&key) {
            let oldvp: &ValueProof = self.key_proof.get(&key).expect("key not present");
            if vp.v == oldvp.v {
                return;
            }
            if !vp.worth_more(oldvp) {
                return;
            }

            self.remove(vp.k);
        }

        let h = vp.h;
        self.hash_key.insert(h, key);
        self.key_proof.insert(key, *vp);

        for b in 0..32 {
            let hpre = h >> b;
            if hpre > 0 {
                if self.prefix_hash.contains_key(&hpre) {
                    let c = self.prefix_hash[&hpre] ^ h;
                    *self.prefix_hash.entry(hpre).or_insert(0) = c;
                } else {
                    self.prefix_hash.insert(hpre, h);
                }
            }
        }
    }
}
