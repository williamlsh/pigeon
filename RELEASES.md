Version 0.1.5 (2022-03-26)
==========================

Enhancements:

- Polling since last tweet.

Version 0.1.4 (2022-03-23)
==========================

Features:

- Add timeline polling support (#7).

Version 0.1.3 (2022-03-23)
==========================

Fixes:

- Fix skip exporting none-timeline data.
- Fix including existing RocksDB column families to open.

Version 0.1.2 (2022-03-23)
==========================

Fixes:

- Fix sync position mark (#4).

Features:

- add minimum request interval to pass Telegram api rate limit.

Version 0.1.1 (2022-03-23)
==========================

Fixes:

- Fix timeline iteration order (#3).

Version 0.1.0 (2022-03-22)
==========================

Features:

- Sync Timeline tweets in RocksDB with Telegram channel (#2).
- Export Timeline raw data from RocksDB (#2).

Version 0.0.2 (2022-03-29)
==========================

Fixes:

- Use timestamp as database key to avoid keys duplication.

Version 0.0.1 (2022-03-12)
==========================

Features:

- Archive twitter user timeline to RocksDB (#1).
- Support multiple users.
