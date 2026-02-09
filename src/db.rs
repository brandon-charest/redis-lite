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

    pub fn rpush(&self, key: String, values: Vec<String>) -> usize {
        let mut lock = self.state.lock().unwrap();

        let entry = lock
            .kv
            .entry(key)
            .or_insert((DataType::List(Vec::new()), None));

        match &mut entry.0 {
            DataType::List(list) => {
                list.extend(values);
                list.len()
            }
            _ => 0,
        }
    }

    pub fn lrange(&self, key: String, start: i64, end: i64) -> Result<Vec<String>, ()> {
        let mut lock = self.state.lock().unwrap();

        match lock.kv.get(&key) {
            Some((DataType::List(list), _expiry)) => {
                let len = list.len() as i64;
                if len == 0 {
                    return Ok(Vec::new());
                }

                let mut start_idx = if start < 0 { len + start } else { start };
                let mut end_idx = if end < 0 { len + end } else { end };

                if start_idx < 0 {
                    start_idx = 0;
                }
                if end_idx >= len {
                    end_idx = len - 1;
                }

                if start_idx >= end_idx || start_idx >= len {
                    return Ok(Vec::new());
                }
                let result = list[start_idx as usize..=end_idx as usize].to_vec();
                Ok(result)
            }
            Some(_) => Err(()),
            None => Ok(Vec::new()),
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

        db.set("temp".to_string(), "val".to_string(), Some(expiry));

        assert!(db.get("temp").is_some());

        thread::sleep(Duration::from_millis(60));

        assert!(db.get("temp").is_none());
    }

    #[test]
    fn test_rpush_list() {
        let db = Db::new();

        let len1 = db.rpush("mylist".to_string(), vec!["a".to_string()]);
        assert_eq!(len1, 1);

        let len2 = db.rpush("mylist".to_string(), vec!["b".to_string(), "c".to_string()]);
        assert_eq!(len2, 3);

        match db.get("mylist") {
            Some(DataType::List(vec)) => {
                assert_eq!(vec, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
            }
            _ => panic!("Expected List"),
        }
    }
}
