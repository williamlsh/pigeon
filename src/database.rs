use rocksdb::{
    ColumnFamilyDescriptor, DBIterator, Direction, IteratorMode, Options, WriteBatch, DB,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Display, path::Path};

use crate::utils;

const COLUMN_FAMILY_TIMELINE_PREFIX: &str = "timeline";
const COLUMN_FAMILY_POLL_PREFIX: &str = "poll";
pub const COLUMN_FAMILY_SYNC_CURSOR: &str = "sync_cursor";
pub const COLUMN_FAMILY_NEWEST_TWEET_ID: &str = "newest_tweet_id";

#[derive(Debug)]
pub struct Database(pub DB);

impl Database {
    pub fn open_with_cfs<P: AsRef<Path>>(path: P, cfs: &[String]) -> Self {
        let mut cf_opts = Options::default();
        cf_opts.set_max_write_buffer_number(32);
        let cfds: Vec<ColumnFamilyDescriptor> = cfs
            .iter()
            .map(|cf| ColumnFamilyDescriptor::new(cf, cf_opts.clone()))
            .collect();

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let db = DB::open_cf_descriptors(&db_opts, path, cfds).expect("could not open rocksdb");
        Self(db)
    }

    pub fn put_cf<K, V>(&self, cf: &str, key: K, value: &V) -> Result<(), String>
    where
        K: AsRef<[u8]>,
        V: Serialize + std::fmt::Debug + ?Sized,
    {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => match serde_json::to_string(value) {
                Ok(serialized) => self
                    .0
                    .put_cf(cf_handle, key, serialized.into_bytes())
                    .map_err(|error| format!("could not put data into rocksdb: {:?}", error)),
                Err(error) => Err(format!("could not serialize to string: {:?}", error)),
            },
            None => Err(format!("no such column family: {}", cf)),
        }
    }

    pub fn put_cf_bytes<K, V>(&self, cf: &str, key: K, value: V) -> Result<(), String>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => self
                .0
                .put_cf(cf_handle, key, value)
                .map_err(|error| format!("could not put data into rocksdb: {:?}", error)),
            None => Err(format!("no such column family: {}", cf)),
        }
    }

    pub fn get_cf<K, T>(&self, cf: &str, key: K) -> Result<Option<T>, String>
    where
        K: AsRef<[u8]>,
        T: DeserializeOwned,
    {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => match self.0.get_cf(cf_handle, key) {
                Ok(value) => match value {
                    Some(bytes) => utils::deserialize_from_bytes(bytes),
                    None => Ok(None),
                },
                Err(error) => Err(format!(
                    "could not get value from column family: {:?}",
                    error
                )),
            },
            None => Err(format!("no such column family: {}", cf)),
        }
    }

    pub fn get_cf_bytes<K: AsRef<[u8]>>(
        &self,
        cf: &str,
        key: K,
    ) -> Result<Option<Vec<u8>>, String> {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => match self.0.get_cf(cf_handle, key) {
                Ok(value) => Ok(value),
                Err(error) => Err(format!(
                    "could not get value from column family: {:?}",
                    error
                )),
            },
            None => Err(format!("no such column family: {}", cf)),
        }
    }

    pub fn delete_cf<K: AsRef<[u8]>>(&self, cf: &str, key: K) -> Result<(), String> {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => self
                .0
                .delete_cf(cf_handle, key)
                .map_err(|error| format!("could not delete key from {}: {:?}", cf, error)),
            None => Err(format!("no such column family: {}", cf)),
        }
    }

    pub fn last_kv_in_cf(
        &self,
        cf: &str,
    ) -> Option<Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>> {
        self.0
            .cf_handle(cf)
            .and_then(|cf_handle| self.0.iterator_cf(cf_handle, IteratorMode::End).next())
    }

    pub fn first_kv_in_cf(
        &self,
        cf: &str,
    ) -> Option<Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>> {
        self.0
            .cf_handle(cf)
            .and_then(|cf_handle| self.0.iterator_cf(cf_handle, IteratorMode::Start).next())
    }

    pub fn cf_timeline_from_username<T: AsRef<str> + Display>(username: T) -> String {
        format!("{}:{}", COLUMN_FAMILY_TIMELINE_PREFIX, username)
    }

    pub fn cf_poll_from_username<T: AsRef<str> + Display>(username: T, timestamp: i64) -> String {
        format!("{}:{}:{}", COLUMN_FAMILY_POLL_PREFIX, username, timestamp)
    }

    pub fn batch_put_cf(&self, cf: &str, kvs: Vec<(&str, &str)>) -> Result<(), String> {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => {
                let mut batch = WriteBatch::default();
                for (key, value) in kvs {
                    batch.put_cf(cf_handle, key, value);
                }
                self.0
                    .write(batch)
                    .map_err(|error| format!("could not write batch: {:?}", error))?;
            }
            None => {
                return Err(format!(
                    "could not get column family handle associated with this name: {}",
                    cf
                ))
            }
        };

        Ok(())
    }

    // Returns a `DBIterator` over a column family since start, optionally from a key forward.
    pub fn iter_cf_since(&self, cf: &str, key: Option<&[u8]>) -> Result<DBIterator, String> {
        let iterator = match self.0.cf_handle(cf) {
            Some(cf_handle) => match key {
                Some(key) => self
                    .0
                    .iterator_cf(cf_handle, IteratorMode::From(key, Direction::Reverse)),
                None => self.0.iterator_cf(cf_handle, IteratorMode::End),
            },
            None => return Err(format!("no such column family: {}", cf)),
        };

        Ok(iterator)
    }

    pub fn list_cf<P: AsRef<Path>>(path: P) -> Result<Vec<String>, String> {
        DB::list_cf(&Options::default(), path)
            .map_err(|error| format!("could not list all column families: {:?}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::utils;
    use rocksdb::{Options, DB};

    #[test]
    fn test_open_with_cfs() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_put_cf() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);
        cfs.iter()
            .enumerate()
            .for_each(|(i, cf)| db.put_cf(cf, &i.to_string(), &i.to_string()).unwrap());

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_get_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        cfs.iter().enumerate().for_each(|(i, cf)| {
            let x = &i.to_string();
            db.put_cf(cf, x, x).unwrap();
            let value: String = db.get_cf(cf, x).unwrap().unwrap();
            assert_eq!(i.to_string(), value);
        });

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_last_kv_in_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        cfs.iter().for_each(|cf| {
            db.put_cf(cf, "a", "b").unwrap();
            db.put_cf(cf, "e", "f").unwrap();
            db.put_cf(cf, "x", "y").unwrap();

            let (_, value) = db.last_kv_in_cf(cf).unwrap().unwrap();
            let value: String = utils::deserialize_from_bytes(value.to_vec())
                .unwrap()
                .unwrap();
            assert_eq!("y".to_string(), value);
        });

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_batch_put_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        cfs.iter().for_each(|cf| {
            db.batch_put_cf(cf, vec![("1", "a"), ("2", "b"), ("3", "c")])
                .unwrap()
        });
    }

    #[test]
    fn test_iter_cf_since_key() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        cfs.iter().for_each(|cf| {
            db.put_cf(cf, "a", "b").unwrap();
            db.put_cf(cf, "e", "f").unwrap();
            db.put_cf(cf, "x", "y").unwrap();

            let iter = db.iter_cf_since(cf, Some("e".as_bytes())).unwrap();
            for (key, value) in iter.flatten() {
                println!(
                    "key: {}, value: {}",
                    String::from_utf8(key.to_vec()).unwrap(),
                    String::from_utf8(value.to_vec()).unwrap()
                );
            }
        });

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_iter_cf_since_none() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        cfs.iter().for_each(|cf| {
            db.put_cf(cf, "a", "b").unwrap();
            db.put_cf(cf, "e", "f").unwrap();
            db.put_cf(cf, "x", "y").unwrap();

            let iter = db.iter_cf_since(cf, None).unwrap();
            for (key, value) in iter.flatten() {
                println!(
                    "key: {}, value: {}",
                    String::from_utf8(key.to_vec()).unwrap(),
                    String::from_utf8(value.to_vec()).unwrap()
                );
            }
        });

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }
}
