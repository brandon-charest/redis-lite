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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_set_and_get_string() {
        let db = Db::new();
        db.set("foo".to_string(), "bar".to_string(), None);

        let result = db.get("foo");
        match result {
            Some(DataType::String(s)) => assert_eq!(s, "bar"),
            _ => panic!("Expected String 'bar'"),
        }
    }

    #[test]
    fn test_expiry_logic() {
        let db = Db::new();
        let expiry = Instant::now() + Duration::from_millis(50);

        // Set key with 50ms expiry
        db.set("temp".to_string(), "val".to_string(), Some(expiry));

        // Immediate fetch should exist
        assert!(db.get("temp").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(60));

        // Fetch should trigger lazy delete and return None
        assert!(db.get("temp").is_none());
    }

    #[test]
    fn test_rpush_list() {
        let db = Db::new();

        // Push 1st item (creates list)
        let len1 = db.rpush("mylist".to_string(), "a".to_string());
        assert_eq!(len1, 1);

        // Push 2nd item (appends)
        let len2 = db.rpush("mylist".to_string(), "b".to_string());
        assert_eq!(len2, 2);

        // Verify content (internal check)
        match db.get("mylist") {
            Some(DataType::List(vec)) => {
                assert_eq!(vec, vec!["a".to_string(), "b".to_string()]);
            }
            _ => panic!("Expected List"),
        }
    }
}
