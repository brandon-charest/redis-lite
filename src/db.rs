use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};

struct DbState {
    kv: HashMap<String, (DataType, Option<Instant>)>,
}

#[derive(Clone, Debug)]
pub enum DataType {
    String(String),
    List(Vec<String>),
    Set(HashSet<String>),
    Hash(HashMap<String, String>),
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

    pub fn get(&self, key: &str) -> Option<DataType> {
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
        let data = DataType::String(value);
        lock.kv.insert(key, (data, expiry));
    }

    pub fn rpush(&self, key: String, value: String) -> usize {
        let mut lock = self.state.lock().unwrap();

        let entry = lock
            .kv
            .entry(key)
            .or_insert((DataType::List(Vec::new()), None));

        match &mut entry.0 {
            DataType::List(list) => {
                list.push(value);
                list.len()
            }
            _ => 0,
        }
    }
}
