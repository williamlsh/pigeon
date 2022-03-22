set dotenv-load

test:
    @cargo test --all-targets -- --test-threads=1

archive:
    @RUST_BACKTRACE=1 RUST_LOG=debug cargo run -r -- \
        archive \
        --twitter-api-token $TWITTER_API_TOKEN \
        --twitter-usernames $TWITTER_USERNAMES \
        --rocksdb-path $ROCKSDB_PATH

export:
    @RUST_BACKTRACE=1 RUST_LOG=debug cargo run -r -- \
        export --rocksdb-path $ROCKSDB_PATH

sync:
    @RUST_BACKTRACE=1 RUST_LOG=debug cargo run -r -- \
        sync \
        --telegram-bot-api-token $TELEGRAM_BOT_API_TOKEN \
        --twitter-usernames $TWITTER_USERNAMES \
        --channel-usernames $CHANNEL_USERNAMES \
        --rocksdb-path $ROCKSDB_PATH

delete-database:
    @rm -rf $ROCKSDB_PATH
