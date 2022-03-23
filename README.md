# Pigeon

Tweets in sync with Telegram channel.

## Features

- Archive Twitter users' timeline.
- Store and export timeline data in and from RocksDB.
- Sync timeline tweets with Telegram channel.

## How to use

General commands with options:

```
$ cargo run -- --help

pigeon 0.1.0

USAGE:
    pigeon <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    archive    Archive users' Twitter timeline raw data to RocksDB
    export     Export all raw data from RocksDB
    help       Print this message or the help of the given subcommand(s)
    sync       Sync tweets extracted from RocksDB raw timeline data to Telegram channel
```

You can also use targets predefined in `justfile`.

Make sure you have [just](https://github.com/casey/just#installation) installed in your machine.

Prepare a `.env` file with environments below:

```
TWITTER_API_TOKEN=<your twitter api token>
TELEGRAM_BOT_API_TOKEN=<your Telegram bot api token>
ROCKSDB_PATH=<RocksDB path>
TWITTER_USERNAMES=<your target Twitter usernames>
CHANNEL_USERNAMES=<your target Telegram channel usernames>
```

For example, to archive users' timeline:

```
just archive
```
