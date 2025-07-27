
# 🚀 tinyredis

> A lightweight Redis reimplementation in Rust — fast, embeddable, and easy to understand.

![Rust](https://img.shields.io/badge/Rust-💛-orange)
![License](https://img.shields.io/github/license/wangyi-kai/tinyredis)
![Status](https://img.shields.io/badge/status-WIP-red)

---

## ✨ Overview

*tinyredis* is a lightweight Redis implementation written in Rust. It is designed to help understand the internal mechanisms of Redis while building a high-performance, clean, and readable in-memory key-value database. It is compatible with the RESP protocol, supports basic data structures, and features asynchronous network I/O and efficient memory management.

---

## 🧱 Features

* Beginner-Friendly: Core data structures and command parsing logic are simple and clear
* Asynchronous Execution: Built on tokio asynchronous networking model
* Multi-Database Support: Compatible with redis multi-DB architecture
* RESP Protocol Parsing: Supports redis native protocol communication
* Modular Design: Easy to extend commands and data structures
* Test Coverage: Includes unit tests

---

## 🚀 Quick Start

### Build
```bash
git clone https://github.com/wangyi-kai/tinyredis.git
cd tinyredis/src
```
### Run tinyredis
```bash
start server
cargo run --release --bin redis_server
start client
cargo run --release --bin redis_cli
default bind address `127.0.0.1:8000`
```

---
## 📦 Support Command
* `hset key field value`
* `hget key field`
* `hdel key field`
* `echo message`
* `ping`
* `select db`
* `setnx key value`
* `setxx key value`
* `strlen key`
* `get key`
*  More commands are being developed...


---

## 🧪 Benchmark

* To be developed

---

## 📚 Architecture Design

```bash
src/
|—— bin/         # Application entry point
├── client/      # tinyredis Client
├── cluster/     # Cluster
├── db/          # Core database structures
├── parser/      # RESP parser
└── server/      # tinyredis Server
```
---

## 🛠️ Technology
* [Rust](https://www.rust-lang.org/)
* [Tokio](https://tokio.rs/)
* [Bytes](https://docs.rs/bytes)
* [Serde](https://serde.rs/)
* [Tracing](https://docs.rs/tracing)
* [Clap](https://docs.rs/clap)


---
## 📈 Future Plan

* [ ] Support for RDB / AOF persistence
* [ ] Publish Docker image
* [ ] Release benchmark tool
* [ ] Implement transactions (MULTI/EXEC)
* [ ] Lua scripting support
* [ ] Cluster protocol compatibility

---
## ❤️ Acknowledgements
* [Redis](https://redis.io/)
* [mini-redis](https://github.com/tokio-rs/mini-redis)
* [kedis-rust](https://github.com/kwsc98/kedis-rust)
---

## 📄 License
* [MIT](LICENSE)
---

## 🗨️ Contact Me
You're welcome to ask questions or start a discussion on GitHub Discussions, or submit an Issue / PR🙌.

