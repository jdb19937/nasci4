use std::collections::BTreeMap;

pub struct HashTree {
  pub key_present : BTreeMap<u64,bool>,
  pub prefix_count : BTreeMap<u64,u64>,
  pub prefix_hash : BTreeMap<u64,u64>,
  pub hash_key : BTreeMap<u64,u64>,
}

pub fn hash(key : u64) -> u64 {
  return (key * key) % 18446744073709551577;
}

impl HashTree {
    pub fn new() -> Self {
        let s = HashTree {
            prefix_count: BTreeMap::<u64,u64>::new(),
            prefix_hash: BTreeMap::<u64,u64>::new(),
            key_present: BTreeMap::<u64,bool>::new(),
            hash_key: BTreeMap::<u64,u64>::new(),
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


    pub fn insert(&mut self, key: u64) {
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
