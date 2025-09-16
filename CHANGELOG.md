#### 2025-09-16
- Upgraded to Rust edition 2024 for latest language features
- Updated all dependencies to 2025 versions:
  - tokio: 1.40 (async runtime improvements)
  - reqwest: 0.12 (better HTTP/3 support)
  - uuid: 1.10 (performance improvements)
  - Other dependencies updated for security and performance
- Updated Docker base image to Rust 1.88-slim
- Improved build times with Docker layer caching

#### 2021-03-15
- Updated internals and libraries. The scraper should be even more efficient now.

#### 2020-04-06
- Added `max_request_retries` field and retrying failed requests in general.

#### 2020-04-04
- Removed sync mode and `run_async` option. Everything is async now.
- Added `max_concurrency` field. This fixes all memory problems with previous async implementation.

#### 2020-02-09
- Added support of async scraping. Can be turned on with `"run_async": true`.
- Added buffering of results before pushing into dataset (to not overwhelm Apify API). Can be changed via `push_data_size`.