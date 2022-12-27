use std::collections::BTreeMap;
use std::cmp::Ordering;

pub fn hash(key : u64) -> u64 {
    return ((key * key + 1) % 4294967291) + 1;
}

pub struct ValueProof {
    pub k : u64,
    pub v : u64,
    pub ts : u64,
    pub seed : u64,
    pub h : u64,
}

impl ValueProof {
    pub fn new() -> Self {
        return ValueProof { k: 0, v: 0, ts: 0, seed: 0, h: 0 };
    }

    pub fn compute_hash(&mut self) {
        self.h = hash(self.k ^ hash(self.v ^ hash(self.ts ^ hash(self.seed)))); 
    }

    pub fn worth_more(&self, other : &ValueProof) -> bool {
        let base = 1.00001;

        // decay = -log(base)
        // 1/h0 * exp(-decay * (now-t0)) > 1/h1 * exp(-decay * (now-t1)) 
        // h1 > h0 * exp(decay * (now-t0) -decay * (now-t1)) 
        // h1 > h0 * exp(decay * ((now-t0)-(now-t1)) 
        // h1 > h0 * exp(decay * (t1 - t0))
        // h0 * exp(decay * (t1 - t0)) < h1

        let h64 = self.h as f64;
        let o64 = other.h as f64;

//println!("ts0={} ts1={}", self.ts, other.ts);

        if self.ts > other.ts {
            let dt : f64 = (self.ts - other.ts) as f64;
//println!("dt={} pf={} h64={h64} o64={o64} h64r={}", dt, f64::powf(base, dt), f64::powf(base, dt) * h64);
            return h64 * f64::powf(base, -dt) < o64;
        } else {
            let dt : f64 = (other.ts - self.ts) as f64;
            return h64 * f64::powf(base, dt) < o64;
        }
    }
}

impl Ord for ValueProof {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.worth_more(other) {
             Ordering::Greater
        } else {
             Ordering::Less
        }
    }
}
impl PartialOrd for ValueProof {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.worth_more(other) {
             Some(Ordering::Greater)
        } else {
             Some(Ordering::Less)
        }
    }
}
impl PartialEq for ValueProof {
    fn eq(&self, other: &Self) -> bool {
        return false;
    }
}
impl Eq for ValueProof { }

pub struct HashTree {
  pub key_present : BTreeMap<u64,bool>,
  pub prefix_count : BTreeMap<u64,u64>,
  pub prefix_hash : BTreeMap<u64,u64>,
  pub hash_key : BTreeMap<u64,u64>,
  pub key_proof : BTreeMap<u64,ValueProof>,
}

impl HashTree {
    pub fn new() -> Self {
        let s = HashTree {
            prefix_count: BTreeMap::<u64,u64>::new(),
            prefix_hash: BTreeMap::<u64,u64>::new(),
            key_present: BTreeMap::<u64,bool>::new(),
            hash_key: BTreeMap::<u64,u64>::new(),
            key_proof: BTreeMap::<u64,ValueProof>::new(),
        };
        return s;
    }

    pub fn lookup(&self, key: u64) -> bool {
        return self.key_present.contains_key(&key);
    }

    pub fn prehash(&self, pre: u64) -> u64 {
        if self.prefix_hash.contains_key(&pre) {
            return *self.prefix_hash.get(&pre).expect("key not present");
        } else {
            return 0;
        }
    }

    pub fn precount(&self, pre: u64) -> u64 {
        if self.prefix_hash.contains_key(&pre) {
            return *self.prefix_count.get(&pre).expect("key not present");
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


    pub fn insert(&mut self, vp : &ValueProof) {
        let key = vp.k;

        if self.lookup(key) {
            return;
        }

        let h = hash(key);
        self.hash_key.insert(h, key);
        self.key_present.insert(key, true);

        for b in 0..63 {
            let hpre = (h & ((1 << b) - 1)) | (1 << b);
            if self.prefix_count.contains_key(&hpre) {
                let c = self.prefix_count[&hpre] + 1;
                *self.prefix_count.entry(hpre).or_insert(0) = c;
            } else {
                self.prefix_count.insert(hpre, 1);
            }
            // println!("inserting b={b} hpre={hpre}");
        }

        for b in 0..63 {
            let hpre = (h & ((1 << b) - 1)) | (1 << b);
            if self.prefix_hash.contains_key(&hpre) {
                let c = self.prefix_hash[&hpre] ^ h;
                *self.prefix_hash.entry(hpre).or_insert(0) = c;
            } else {
                self.prefix_hash.insert(hpre, h);
            }
        }
    }
}
