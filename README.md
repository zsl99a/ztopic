# 🎈 Helium (ztopic)

> 基于 **Rust + Tokio** 的轻量级异步流式主题发布订阅（Pub/Sub）与事件分发组件。

[![Rust](https://img.shields.io/badge/language-Rust_2021-DEA584?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/async-Tokio_1.0-000000?style=flat-square&logo=rust)](https://tokio.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

---

## 📖 项目简介

`Helium`（ztopic）是一个面向高并发异步场景设计的流式发布/订阅与主题管理库。基于 Rust 的零成本抽象与 `futures::Stream` 机制，提供多生产者-多消费者模式下的多路流分发、批量缓冲（Batch Buffer）与 Axum WebSocket 实时推送能力。

---

## ✨ 核心特性

- ⚡ **异步流式驱动**：基于 `tokio` 与 `futures::Stream`，支持非阻塞式异步数据流消费与背压缓冲。
- 🎯 **主题管理器 (`TopicManager`)**：轻量级统一管理多个动态 Topic，支持泛型状态（Store）共享与多消费者分发。
- 📦 **流缓冲与批处理 (`SharedStream / Buffer`)**：内置全局容量与批处理调节机制，支持高效的分批合并与消息路由。
- 🌐 **Web & WebSocket 集成**：原生适配 `Axum` 框架与 WebSocket 协议，方便快速搭建实时流式推送服务端。
- 🛡️ **并发安全**：采用 `parking_lot::Mutex` 与 `Arc`，在保证多线程高并发安全的同时最小化锁开销。

---

## 🛠️ 快速上手

### 添加依赖

```toml
[dependencies]
helium = { git = "https://github.com/zsl99a/ztopic.git" }
tokio = { version = "1", features = ["full"] }
```

### 基础使用示例

```rust
use helium::{Topic, TopicManager};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    // 初始化 TopicManager
    let manager = TopicManager::new(());
    println!("Topic manager initialized: {:?}", manager);
}
```

---

## 📂 模块结构

```text
src/
├── buffer.rs     # 消息流缓冲与队列管理
├── empty.rs      # 空流处理与缺省状态抽象
├── routes.rs     # Web 路由与服务分发接口
├── stream.rs     # 共享数据流（SharedStream）与适配器
├── time.rs       # 定时触发与时间间隔驱动
├── topic.rs      # TopicManager 与核心发布订阅逻辑
└── bin/          # 压力测试、Web 实例与流式运行样例
```

---

## 📄 开源协议

本项目基于 [MIT License](LICENSE) 协议开源。
