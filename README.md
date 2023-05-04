# Pigeon

A Convenient Tool for Syncing Tweets to Telegram Channels.

Pigeon is a powerful tool written in pure Rust that allows you to seamlessly sync Tweets to Telegram channel(s). With its user-friendly features and efficient functionality, Pigeon simplifies the process of keeping your Telegram channels up-to-date with the latest Twitter content.

## Key Features

- Poll Twitter timelines
- Store and display data using RocksDB
- Push Tweets to Telegram channels
- Interruptible and resumable pushing
- No limits on the number of Twitter users and Telegram channels

## Configuration?

To configure Pigeon, refer to the provided [config.toml](config.toml) file for an example setup. Additionally, you'll need to obtain the following API tokens:

- Twitter API token: Visit "[How to get access to the Twitter API](https://developer.twitter.com/en/docs/twitter-api/getting-started/getting-access-to-the-twitter-api)" for instructions on obtaining this token.
- Telegram Bot API token: Follow the guide on "[Creating a new bot](https://core.telegram.org/bots/features#creating-a-new-bot.)" to acquire the necessary token.

## Usage

To build the Pigeon binary, use the following command:

```
cargo build --release
```

General commands:

```
$ target/release/pigeon --help
Usage: pigeon [OPTIONS] --config-path <config.toml> <COMMAND>

Commands:
  poll  Poll Twitter users' timeline
  push  Push timeline to Telegram channel(s)
  info  Display overview information about Database
  help  Print this message or the help of the given subcommand(s)

Options:
  -d, --debug                      Activate debug mode
  -c, --config-path <config.toml>  Config file path
  -h, --help                       Print help information
```

Alternatively, you can download the pre-built binary from the latest [release](https://github.com/williamlsh/pigeon/releases) or utilize the [Pigeon](https://github.com/users/williamlsh/packages/container/package/pigeon) Docker image.

## Proxy Support

If you require network proxy usage, build Pigeon with `socks` feature enabled:

```
cargo build --release --features socks
```

You can set up an HTTP/HTTPS or Socks5 proxy for all network connections through environment variables. For example:

To use an HTTP proxy:

```
export HTTP_PROXY=http://secure.example
```

To use a Socks5 proxy:

```
export https_proxy=socks5://127.0.0.1:1086
```

## Local Data

Pigeon stores all tweet data locally in RocksDB, which resides in the specified path during runtime. Tweets pushed to Telegram channel(s) are automatically deleted, ensuring no unnecessary data clutters your disk storage.

## Author

Pigeon was developed by [William](https://github.com/williamlsh), offering a robust solution for syncing Tweets to Telegram channels efficiently and effortlessly.

## License

MIT License
