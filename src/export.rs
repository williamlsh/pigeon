use crate::{cli::Export, database, twitter::timeline::Timeline, utils};
use log::info;
use rocksdb::{IteratorMode, Options, DB};

pub fn export(args: Export) {
    let opts = Options::default();
    let cfs = DB::list_cf(&opts, &args.rocksdb_path).unwrap();
    info!("All column families: {:?}", &cfs);

    let db = DB::open_cf_for_read_only(&opts, &args.rocksdb_path, &cfs, false).unwrap();
    for cf in cfs {
        if cf.eq(database::COLUMN_FAMILY_SYNC_CURSOR)
            || cf.eq(database::COLUMN_FAMILY_NEWEST_TWEET_ID)
        {
            continue;
        }
        info!("Exporting data in {}", cf);

        let cf_handle = db.cf_handle(&cf).unwrap();
        let iter = db.iterator_cf(cf_handle, IteratorMode::Start);
        iter.for_each(|(_, value)| {
            let timeline: Timeline = utils::deserialize_from_bytes(value.to_vec())
                .unwrap()
                .unwrap();
            println!("{:?}", timeline);
        });
        info!("Finished exporting data in {}", cf);
    }
}
