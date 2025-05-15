use alloc::vec;
use alloc::vec::Vec;
use axhal::misc::random;
use core::hash::{Hash, Hasher};

// 写一个简化的HashMap，只完成arceos/exercises/support_hashmap/src/main.rs中用到的功能
pub struct HashMap<K, V> {
    seed: u64,
    buckets: Vec<Vec<(K, V)>>,
    bucket_count: usize,
    size: usize
}

pub struct Iter<'a, K, V> {
    buckets: &'a [Vec<(K, V)>],
    index1: usize,
    index2: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where K: Clone, V: Clone
{
    type Item = &'a (K, V);
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index1 >= self.buckets.len() {
                return None;
            }
            if self.index2 >= self.buckets[self.index1].len() {
                self.index1 += 1;
                self.index2 = 0;
                continue;
            }
            let res = Some(&self.buckets[self.index1][self.index2]);
            self.index2 += 1;
            return res;
        }
    }
}

/* chatgpt教的，Hasher trait 和 Hash trait 结合使用。
调用 key.hash(&mut hasher)， key 会使用 hasher 的 write() 往 hasher 里写入字节，调用 finish() 得到最终结果(想要的hash值)
*/
struct SimpleHasher {
    hash: u64,
}

impl Hasher for SimpleHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash = self.hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

impl SimpleHasher {
    fn new(seed: u64) -> Self {
        Self { hash: seed }
    }
}

impl<'a, K, V> HashMap<K, V>
where K: Hash + Eq + Clone, V: Clone
{
    pub fn new() -> Self {
        Self {
            seed: random() as u64,
            buckets: vec![Vec::new(); 16],
            bucket_count: 16,
            size: 0
        }
    }

    fn hash(&self, key: &K) -> u64 {
        /* 用这个会巨慢：
        let hash_value = unsafe {
            key as *const K as u64
        };
        (hash_value + self.seed) % (self.bucket_count as u64) */
        let mut hasher = SimpleHasher::new(self.seed); // self.seed初始化后不能变，否则同一个key两次hash()结果会不一样
        key.hash(&mut hasher); // Types implementing Hash are able to be hashed with an instance of Hasher。Hash和Hasher都是个trait
        let hash_value = hasher.finish();
        hash_value % (self.bucket_count as u64)
    }

    pub fn insert(&mut self, key: K, value: V) {
        let index = self.hash(&key);
        let bucket = &mut self.buckets[index as usize];
        for p in bucket.iter_mut() {
            if p.0 == key {
                p.1 = value;
                return;
            }
        }
        bucket.push((key, value));
        self.size += 1;
        if self.size > self.bucket_count * 2 {
            self.rehash();
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn iter(&'a self) -> Iter<'a, K, V> {
        Iter {
            buckets: &self.buckets,
            index1: 0,
            index2: 0
        }
    }

    /// 给 self.buckets 扩容
    fn rehash(&mut self) {
        let old = core::mem::take(&mut self.buckets);
        self.bucket_count *= 2;
        self.buckets = vec![Vec::new(); self.bucket_count];
        for old_bucket in old {
            for (key, value) in old_bucket {
                self.insert(key, value);
            }
        }
    }
}