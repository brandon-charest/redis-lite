use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::oneshot;

struct DbState {
    kv: HashMap<String, (DataType, Option<Instant>)>,
    waiting: HashMap<String, VecDeque<oneshot::Sender<String>>>,
}

#[derive(Clone, Debug)]
pub enum DataType {
    String(String),
    List(VecDeque<String>),
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
            state: Arc::new(Mutex::new(DbState {
                kv: HashMap::new(),
                waiting: HashMap::new(),
            })),
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

    pub fn rpush(&self, key: String, values: Vec<String>) -> Result<usize, ()> {
        self.modify_list(key, |list| {
            list.extend(values);
        })
    }

    pub fn lpush(&self, key: String, values: Vec<String>) -> Result<usize, ()> {
        self.modify_list(key, |list| {
            for value in values {
                list.push_front(value);
            }
        })
    }

    pub fn blpop_register(&self, key: String) -> Result<Option<String>, oneshot::Receiver<String>> {
        let mut lock = self.state.lock().unwrap();
        if let Some((DataType::List(list), _)) = lock.kv.get_mut(&key) {
            if let Some(val) = list.pop_front() {
                if list.is_empty() {
                    lock.kv.remove(&key);
                }
                return Ok(Some(val));
            }
        }

        let (tx, rx) = oneshot::channel();
        lock.waiting
            .entry(key)
            .or_insert_with(VecDeque::new)
            .push_back(tx);

        Err(rx)
    }

    fn modify_list<F>(&self, key: String, op: F) -> Result<usize, ()>
    where
        F: FnOnce(&mut VecDeque<String>),
    {
        let mut lock = self.state.lock().unwrap();
        let state = &mut *lock;

        let mut is_empty = false;
        let mut pushed_len = 0;
        let mut is_wrong_type = false;
        {
            let entry = state
                .kv
                .entry(key.clone())
                .or_insert((DataType::List(VecDeque::new()), None));

            if let DataType::List(list) = &mut entry.0 {
                op(list);
                pushed_len = list.len();

                if let Some(waiters) = state.waiting.get_mut(&key) {
                    while !list.is_empty() && !waiters.is_empty() {
                        if let Some(sender) = waiters.pop_front() {
                            if let Some(val) = list.pop_front() {
                                let _ = sender.send(val);
                            }
                        }
                    }
                }
                is_empty = list.is_empty();
            } else {
                is_wrong_type = true;
            }
        }

        if is_wrong_type {
            return Err(());
        }

        if is_empty {
            state.kv.remove(&key);
        }

        Ok(pushed_len)
    }

    pub fn lrange(&self, key: String, start: i64, end: i64) -> Result<Vec<String>, ()> {
        let lock = self.state.lock().unwrap();

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
                let result: Vec<String> = list
                    .range(start_idx as usize..=end_idx as usize)
                    .cloned()
                    .collect();
                Ok(result)
            }
            Some(_) => Err(()),
            None => Ok(Vec::new()),
        }
    }

    pub fn llen(&self, key: String) -> Result<usize, ()> {
        let mut lock = self.state.lock().unwrap();

        if let Some((_, Some(expiry))) = lock.kv.get(&key) {
            if std::time::Instant::now() > *expiry {
                lock.kv.remove(&key);
                return Ok(0);
            }
        }

        match lock.kv.get(&key) {
            Some((DataType::List(list), _)) => Ok(list.len()),
            None => Ok(0),
            Some(_) => Err(()),
        }
    }

    pub fn lpop(&self, key: &str, count: Option<usize>) -> Result<Option<Vec<String>>, ()> {
        let mut lock = self.state.lock().unwrap();

        if let Some((_, Some(expiry))) = lock.kv.get(key) {
            if std::time::Instant::now() > *expiry {
                lock.kv.remove(key);
                return Ok(None);
            }
        }

        match lock.kv.get_mut(key) {
            Some((DataType::List(list), _)) => {
                let needed = count.unwrap_or(1);
                let actual = std::cmp::min(list.len(), needed);

                if actual == 0 {
                    return Ok(None);
                }

                let items: Vec<String> = list.drain(0..actual).collect();
                if list.is_empty() {
                    lock.kv.remove(key);
                }

                Ok(Some(items))
            }
            Some(_) => Err(()),
            None => Ok(None),
        }
    }

    pub fn get_type(&self, key: &str) -> String {
        let mut lock = self.state.lock().unwrap();

        if let Some((_, Some(expiry))) = lock.kv.get(key) {
            if std::time::Instant::now() > *expiry {
                lock.kv.remove(key);
                return "none".to_string();
            }
        }

        match lock.kv.get(key) {
            Some((DataType::String(_), _)) => "string".to_string(),
            Some((DataType::List(_), _)) => "list".to_string(),
            Some((DataType::Set(_), _)) => "set".to_string(),
            Some((DataType::Hash(_), _)) => "hash".to_string(),
            None => "none".to_string(),
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
        assert_eq!(len1, Ok(1));

        let len2 = db.rpush("mylist".to_string(), vec!["b".to_string(), "c".to_string()]);
        assert_eq!(len2, Ok(3));

        match db.get("mylist") {
            Some(DataType::List(vec)) => {
                assert_eq!(vec, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
            }
            _ => panic!("Expected List"),
        }
    }
}
