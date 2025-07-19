
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
* `HSET key field value`
* `HGET key field`
* `HDEL key field`
* `echo message`
* `ping`
* `select db`
* `setnx key value`
* `setxx key value`
* `strlen key`
* `get key`
* 更多命令持续开发中...


---

## 🧪 Benchmark

* To be developed

---

## 📚 Architecture Design

```bash
src/
|—— bin/         # 启动入口
├── client/      # 客户端实现
├── cluster/     # 集群相关
├── db/          # 数据库与数据结构实现
├── parser/      # RESP 协议解析器
└── server/      # 服务端实现
```
---

## 🛠️ 技术栈
* [Rust](https://www.rust-lang.org/)
* [Tokio](https://tokio.rs/)
* [Bytes](https://docs.rs/bytes)
* [Serde](https://serde.rs/)
* [Tracing](https://docs.rs/tracing)
* [Clap](https://docs.rs/clap)


---
## 📈 未来计划

* [ ] 支持 RDB / AOF 持久化
* [ ] 发布 Docker 镜像
* [ ] 发布 benchmark 工具
* [ ] 实现事务（MULTI/EXEC）
* [ ] Lua 脚本支持
* [ ] 集群协议兼容

---
## ❤️ 致谢
* [Redis](https://redis.io/)
* [mini-redis](https://github.com/tokio-rs/mini-redis)
* [kedis-rust](https://github.com/kwsc98/kedis-rust)
---

## 📄 License
* [MIT](LICENSE)
---

## 🗨️ 联系我
欢迎在 [GitHub Discussions](https://github.com/wangyi-kai/tinyredis/discussions) 提问交流, 或提交 Issue / PR 🙌

