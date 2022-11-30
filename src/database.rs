use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct Database(DB);

impl Database {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Self {
        let cfds: Vec<ColumnFamilyDescriptor> = vec!["timeline", "state"]
            .iter()
            .map(|&cf| ColumnFamilyDescriptor::new(cf, Options::default()))
            .collect();

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let db = DB::open_cf_descriptors(&db_opts, path, cfds).expect("could not open rocksdb");
        Self(db)
    }

    pub(crate) fn put_cf<K, V>(&self, cf: &str, key: K, value: V) -> Result<(), String>
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

    pub(crate) fn get_cf<K: AsRef<[u8]>>(
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
}

#[cfg(test)]
mod tests {
    use super::Database;
    use rocksdb::{Options, DB};

    #[test]
    fn open() {
        let db = Database::open("test");
        drop(db);
        DB::destroy(&Options::default(), "test").unwrap();
    }
}
