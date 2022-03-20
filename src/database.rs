use crate::utils;
use log::{error, info};
use rocksdb::{
    AsColumnFamilyRef, ColumnFamily, ColumnFamilyDescriptor, DBIterator, Direction, IteratorMode,
    Options, DB,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::Path};

const COLUMN_FAMILY_TIMELINE_PREFIX: &str = "timeline";

#[derive(Debug)]
pub struct Database(DB);

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

    pub fn get_cf_handles<'a>(&self, cfs: &'a [String]) -> HashMap<&'a str, Option<&ColumnFamily>> {
        cfs.iter()
            .map(|cf| (cf.as_str(), self.0.cf_handle(cf)))
            .collect()
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

    pub fn last_kv_pair_in_cfs<'a, T: DeserializeOwned>(
        &self,
        cfs: &'a [String],
    ) -> HashMap<&'a str, Option<T>> {
        cfs.iter()
            .map(|cf| match self.0.cf_handle(cf) {
                Some(cf_handle) => match self.0.iterator_cf(cf_handle, IteratorMode::End).next() {
                    Some((_, value)) => match utils::deserialize_from_bytes(value.to_vec()) {
                        Ok(value) => (cf.as_str(), value),
                        Err(error) => {
                            error!("could not deserialize from bytes: {:?}", error);
                            (cf.as_str(), None)
                        }
                    },
                    None => {
                        info!("no last found item from {}", cf);
                        (cf.as_str(), None)
                    }
                },
                None => (cf.as_str(), None),
            })
            .collect()
    }

    pub fn cf_timeline_from_user_id(user_id: &str) -> String {
        format!("{}:{}", COLUMN_FAMILY_TIMELINE_PREFIX, user_id)
    }

    // Returns a `DBIterator` over a column family since start, optionally from a key forward.
    fn iter_cf_since(&self, cf_handle: &impl AsColumnFamilyRef, key: Option<&[u8]>) -> DBIterator {
        match key {
            Some(key) => self
                .0
                .iterator_cf(cf_handle, IteratorMode::From(key, Direction::Forward)),
            None => self.0.iterator_cf(cf_handle, IteratorMode::Start),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use rocksdb::{Options, DB};
    use std::collections::HashMap;

    #[test]
    fn test_open_with_cfs() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles = db.get_cf_handles(&cfs);
        cf_handles
            .into_values()
            .for_each(|cf_handle| assert!(cf_handle.is_some()));

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_put_cf() {
        let cfs = [String::from("1"), String::from("2")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles = db.get_cf_handles(&cfs);

        let cf_handles_1 = cf_handles.get("1").unwrap().unwrap();
        db.put_cf(cf_handles_1, "x", "y").unwrap();
        let cf_handles_2 = cf_handles.get("2").unwrap().unwrap();
        db.put_cf(cf_handles_2, "a", "b").unwrap();

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_get_cf() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles = db.get_cf_handles(&cfs);

        let cf_handles_1 = cf_handles.get("1").unwrap().unwrap();
        db.put_cf(cf_handles_1, "x", "y").unwrap();

        let value: String = db.get_cf(cf_handles_1, "x").unwrap().unwrap();
        assert_eq!("y", value);

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_last_kv_pair_in_cfs() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles = db.get_cf_handles(&cfs);

        let cf_handles_1 = cf_handles.get("1").unwrap().unwrap();
        db.put_cf(cf_handles_1, "a", "b").unwrap();
        db.put_cf(cf_handles_1, "e", "f").unwrap();
        db.put_cf(cf_handles_1, "x", "y").unwrap();

        let items_map: HashMap<&str, Option<String>> = db.last_kv_pair_in_cfs(&cfs);
        assert!(items_map.get("1").unwrap().as_ref().unwrap().eq("y"));

        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }

    #[test]
    fn test_iter_cf_since_key() {
        let cfs = [String::from("1")];
        let db = Database::open_with_cfs("test", &cfs);
        let cf_handles = db.get_cf_handles(&cfs);

        let cf_handles_1 = cf_handles.get("1").unwrap().unwrap();
        db.put_cf(cf_handles_1, "a", "b").unwrap();
        db.put_cf(cf_handles_1, "e", "f").unwrap();
        db.put_cf(cf_handles_1, "x", "y").unwrap();

        let iter = db.iter_cf_since(cf_handles_1, Some("e".as_bytes()));
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
        let cf_handles = db.get_cf_handles(&cfs);

        let cf_handles_1 = cf_handles.get("1").unwrap().unwrap();
        db.put_cf(cf_handles_1, "a", "b").unwrap();
        db.put_cf(cf_handles_1, "e", "f").unwrap();
        db.put_cf(cf_handles_1, "x", "y").unwrap();

        let iter = db.iter_cf_since(cf_handles_1, None);
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
