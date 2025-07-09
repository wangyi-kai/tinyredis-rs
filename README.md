# 🚀 tinyredis

> A lightweight Redis reimplementation in Rust — fast, embeddable, and easy to understand.

![Rust](https://img.shields.io/badge/Rust-💛-orange)
![License](https://img.shields.io/github/license/wangyi-kai/tinyredis)
![Status](https://img.shields.io/badge/status-WIP-red)

---

## ✨ 项目简介

**tinyredis** 是一个用 Rust 编写的 Redis 重实现，旨在学习 Redis 内部机制，并构建一个高性能、简洁易读的内存键值数据库。它兼容 RESP 协议，支持基础数据结构，并具备异步网络 IO 和高效内存管理。

---

## 🧱 Features

- 🧠 **学习友好**：核心数据结构和命令解析逻辑简洁明了
- ⚡ **异步运行**：基于 `tokio` 的异步网络模型
- 🧵 **多数据库支持**：兼容 Redis 的多 DB 架构
- 💾 **RESP 协议解析**：支持 Redis 原生协议通信
- 🔧 **模块化设计**：便于扩展指令与数据结构
- 🧪 **测试覆盖**：包含单元测试和基准测试

---

## 🚀 快速开始

### 构建项目

```bash
git clone https://github.com/wangyi-kai/tinyredis.git
cd tinyredis
cargo build --release
