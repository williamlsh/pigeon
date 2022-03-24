use crate::utils;
use rocksdb::{
    AsColumnFamilyRef, ColumnFamily, ColumnFamilyDescriptor, DBIterator, Direction, IteratorMode,
    Options, WriteBatch, DB,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Display, path::Path};

const COLUMN_FAMILY_TIMELINE_PREFIX: &str = "timeline";
pub const COLUMN_FAMILY_USER_INFO: &str = "user_info";
pub const COLUMN_FAMILY_SYNC_CURSOR: &str = "sync_cursor";

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

    pub fn cf_handle(&self, cf: &str) -> Option<&ColumnFamily> {
        self.0.cf_handle(cf)
    }

    pub fn put_cf<T>(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        key: &str,
        value: &T,
    ) -> Result<(), String>
    where
        T: Serialize + std::fmt::Debug + ?Sized,
    {
        match serde_json::to_string(value) {
            Ok(serialized) => self
                .0
                .put_cf(cf_handle, key, serialized.into_bytes())
                .map_err(|error| format!("could not put data into rocksdb: {:?}", error)),
            Err(error) => Err(format!("could not serialize to string: {:?}", error)),
        }
    }

    pub fn get_cf<T: DeserializeOwned>(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        key: &str,
    ) -> Result<Option<T>, String> {
        match self.0.get_cf(cf_handle, key) {
            Ok(value) => match value {
                Some(bytes) => utils::deserialize_from_bytes(bytes),
                None => Ok(None),
            },
            Err(error) => Err(format!(
                "could not get value from column family: {:?}",
                error
            )),
        }
    }

    pub fn last_kv_in_cf(&self, cf: &str) -> Option<(Box<[u8]>, Box<[u8]>)> {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => self.0.iterator_cf(cf_handle, IteratorMode::End).next(),
            None => None,
        }
    }

    pub fn cf_timeline_from_username<T: AsRef<str> + Display>(username: T) -> String {
        format!("{}:{}", COLUMN_FAMILY_TIMELINE_PREFIX, username)
    }

    pub fn batch_put_cf(&self, cf: &str, kvs: Vec<(&str, &str)>) -> Result<(), String> {
        let cf_handle = match self.cf_handle(cf) {
            Some(cf_handle) => cf_handle,
            None => {
                return Err(format!(
                    "could not get column family handle associated with this name: {}",
                    cf
                ))
            }
        };

        let mut batch = WriteBatch::default();
        for (key, value) in kvs {
            batch.put_cf(cf_handle, key, value);
        }
        self.0
            .write(batch)
            .map_err(|error| format!("could not write batch: {:?}", error))?;

        Ok(())
    }

    // Returns a `DBIterator` over a column family since start, optionally from a key forward.
    pub fn iter_cf_since(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        key: Option<&[u8]>,
    ) -> DBIterator {
        match key {
            Some(key) => self
                .0
                .iterator_cf(cf_handle, IteratorMode::From(key, Direction::Reverse)),
            None => self.0.iterator_cf(cf_handle, IteratorMode::End),
        }
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
    use rocksdb::{ColumnFamily, Options, DB};

    #[test]
    fn test_open_with_cfs() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles: Vec<Option<&ColumnFamily>> =
            cfs.iter().map(|cf| db.cf_handle(cf)).collect();
        cf_handles
            .iter()
            .for_each(|cf_handle| assert!(cf_handle.is_some()));

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_put_cf() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles: Vec<Option<&ColumnFamily>> =
            cfs.iter().map(|cf| db.cf_handle(cf)).collect();

        cf_handles.iter().enumerate().for_each(|(i, cf_handle)| {
            db.put_cf(cf_handle.unwrap(), &i.to_string(), &i.to_string())
                .unwrap()
        });

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_get_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        let cf_handle = db.cf_handle("1").unwrap();
        db.put_cf(cf_handle, "x", "y").unwrap();

        let value: String = db.get_cf(cf_handle, "x").unwrap().unwrap();
        assert_eq!("y", value);

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_last_kv_in_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        let cf_handle = db.cf_handle("1").unwrap();
        db.put_cf(cf_handle, "a", "b").unwrap();
        db.put_cf(cf_handle, "e", "f").unwrap();
        db.put_cf(cf_handle, "x", "y").unwrap();

        let (_, value) = db.last_kv_in_cf("1").unwrap();
        let deserialized: String = utils::deserialize_from_bytes(value.to_vec())
            .unwrap()
            .unwrap();
        assert_eq!("y".to_string(), deserialized);

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_batch_put_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        db.batch_put_cf("1", vec![("1", "a"), ("2", "b"), ("3", "c")])
            .unwrap();
    }

    #[test]
    fn test_iter_cf_since_key() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        let cf_handles = db.cf_handle("1").unwrap();
        db.put_cf(cf_handles, "a", "b").unwrap();
        db.put_cf(cf_handles, "e", "f").unwrap();
        db.put_cf(cf_handles, "x", "y").unwrap();

        let iter = db.iter_cf_since(cf_handles, Some("e".as_bytes()));
        for (key, value) in iter {
            println!(
                "key: {}, value: {}",
                String::from_utf8(key.to_vec()).unwrap(),
                String::from_utf8(value.to_vec()).unwrap()
            );
        }

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_iter_cf_since_none() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);

        let cf_handles = db.cf_handle("1").unwrap();
        db.put_cf(cf_handles, "a", "b").unwrap();
        db.put_cf(cf_handles, "e", "f").unwrap();
        db.put_cf(cf_handles, "x", "y").unwrap();

        let iter = db.iter_cf_since(cf_handles, None);
        for (key, value) in iter {
            println!(
                "key: {}, value: {}",
                String::from_utf8(key.to_vec()).unwrap(),
                String::from_utf8(value.to_vec()).unwrap()
            );
        }

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }
}
