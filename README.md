# Pigeon

A convenient tool to sync Tweets to Telegram channel(s) written in pure Rust.

## Features

- Poll Twitter users' timeline.
- Store and display timeline data in and from RocksDB.
- Push timeline tweets to Telegram channel(s).

## How to configure?

See [config.toml](config.toml) for example.

To get a Twitter API token, see: [How to get access to the Twitter API](https://developer.twitter.com/en/docs/twitter-api/getting-started/getting-access-to-the-twitter-api).

To get a Telegram Bot API token, see: [Creating a new bot](https://core.telegram.org/bots/features#creating-a-new-bot).

## How to use

Build binary:

```
cargo build --release
```

General commands:

```
$ target/release/pigeon --help

Usage: pigeon --config-path <config.toml> <COMMAND>

Commands:
  poll  Poll Twitter users' timeline
  push  Push timeline to Telegram channel(s)
  info  Display overview information about Database
  help  Print this message or the help of the given subcommand(s)

Options:
  -c, --config-path <config.toml>  Config file path
  -h, --help                       Print help information
```

You can also download pre-built binary from latest [release](https://github.com/williamlsh/pigeon/releases) or use a [Pigeon](https://github.com/users/williamlsh/packages/container/package/pigeon) Docker image instead.

## Proxy

You can specify HTTP/HTTPS or Socks5 proxy for all network connections from Pigeon through environment variables. For instance:

To use an HTTP proxy:

```
export HTTP_PROXY=http://secure.example
```

Or to use an Socks5 proxy:

```
export https_proxy=socks5://127.0.0.1:1086
```

## Local data

All tweets data is stored in RocksDB in a local path that you specified when running Pigeon. Tweets that are pushed to Telegram channel(s) will automatically be deleted. No garbage data will remain in your disk storage.

## Author

- [William](https://github.com/williamlsh)
