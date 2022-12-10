use anyhow::{bail, Context, Result};
use rocksdb::{ColumnFamilyDescriptor, DBIterator, IteratorMode, Options, DB};
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

    pub(crate) fn put_cf<K, V>(&self, cf: &str, key: K, value: V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => Ok(self.0.put_cf(cf_handle, key, value)?),
            None => bail!("no such column family: {}", cf),
        }
    }

    pub(crate) fn get_cf<K: AsRef<[u8]>>(&self, cf: &str, key: K) -> Result<Option<Vec<u8>>> {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => self
                .0
                .get_cf(cf_handle, key)
                .with_context(|| "could not get value from column family"),
            None => bail!("no such column family: {}", cf),
        }
    }

    pub(crate) fn iterator_cf(&self, cf: &str) -> Option<DBIterator> {
        self.0
            .cf_handle(cf)
            .map(|cf_handle| self.0.iterator_cf(cf_handle, IteratorMode::Start))
    }

    /// Performs an `from` inclusive but `to` exclusive range (`["from", "to")`) deletion.
    pub(crate) fn delete_range_cf<K>(&self, cf: &str, from: K, to: K) -> Result<()>
    where
        K: AsRef<[u8]>,
    {
        match self.0.cf_handle(cf) {
            Some(cf_handle) => Ok(self.0.delete_range_cf(cf_handle, from, to)?),
            None => bail!("no such column family: {}", cf),
        }
    }

    pub(crate) fn drop_cf(&mut self, cf: &str) -> Result<()> {
        Ok(self.0.drop_cf(cf)?)
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
