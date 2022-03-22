use crate::{cli::Export, twitter::timeline::Timeline, utils};
use log::info;
use rocksdb::{IteratorMode, Options, DB};

pub fn export(args: Export) {
    let opts = Options::default();
    let cfs = DB::list_cf(&opts, &args.rocksdb_path).unwrap();
    info!("Exporting data in {:?}", &cfs);

    let db = DB::open_cf_for_read_only(&opts, &args.rocksdb_path, &cfs, false).unwrap();
    for cf in cfs {
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
