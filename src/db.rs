use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

struct DbState {
    kv: HashMap<String, (String, Option<Instant>)>,
}

#[derive(Clone)]
pub struct Db {
    state: Arc<Mutex<DbState>>,
}

impl Db {
    pub fn new() -> Db {
        Db {
            state: Arc::new(Mutex::new(DbState { kv: HashMap::new() })),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut lock = self.state.lock().unwrap();

        if let Some((_val, Some(expiry))) = lock.kv.get(key) {
            if Instant::now() > *expiry {
                lock.kv.remove(key);
                return None;
            }
        }

        lock.kv.get(key).map(|(val, _)| val.clone())
    }

    pub fn set(&self, key: String, value: String, expiry: Option<Instant>) {
        let mut lock = self.state.lock().unwrap();
        lock.kv.insert(key, (value, expiry));
    }
}
