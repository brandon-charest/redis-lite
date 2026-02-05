use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct DbState {
    kv: HashMap<String, String>,
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
        let lock = self.state.lock().unwrap();
        lock.kv.get(key).cloned()
    }

    pub fn set(&self, key: String, value: String) {
        let mut lock = self.state.lock().unwrap();
        lock.kv.insert(key, value);
    }
}
