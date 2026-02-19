# MemoryOS-Rust

High-Performance AI Agent Memory Management System - Rust Implementation

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.93+-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-Production_Ready-brightgreen.svg)](./CHANGELOG.md)
[![Tests](https://img.shields.io/badge/Tests-15/15_Passing-brightgreen.svg)](./CHANGELOG.md)

**Languages**: [English](README.md) | [简体中文](README_CN.md) | [日本語](README_JA.md) | [Français](README_FR.md) | [العربية](README_AR.md) | [Deutsch](README_DE.md) | [Español](README_ES.md) | [한국어](README_KO.md)

---

## 🎯 Overview

MemoryOS-Rust is a high-performance AI Agent memory management system built with Rust + Tokio, featuring a 3-Tier memory architecture (STM/MTM/LTM), OpenAI API compatibility, and support for 100,000+ concurrent users.

---

## ✨ Key Features

- 🚀 **High Performance**: Rust + Tokio, supporting high concurrency with 10K+ QPS per instance.
- 🧠 **3-Tier Memory**: STM (Redis/NATS) → MTM (Qdrant) → LTM (Qdrant).
- 💾 **Dual Storage Options**: Redis (default, ~1ms) or NATS (alternative, ~2ms, no warnings).
- ⚡ **FAQ Direct Hit**: High-frequency Q&A auto-promoted to instant response (< 50ms).
- 🔌 **Universal Gateway**: OpenAI protocol compatible, supports Gemini, Claude, Ollama, DeepSeek, Azure.
- 🕸️ **Graph Memory**: **Qdrant-Native GraphRAG** with Mermaid visualization.
- 📚 **Knowledge Export**: Auto-export FAQs to Wiki (S3/Confluence), supports **Agent Playbook**.
- 🛡️ **Enterprise Security**: RBAC, PII sanitization, prompt injection defense, GDPR right to be forgotten.
- 🤖 **Smart Routing**: Auto-route between local Llama (hot/private) and cloud GPT-4 (complex/cold).
- 🎯 **6 Performance Optimizations**: Bloom Filter, LRU Cache, Batch Processing, Heat Buffer, Similarity Filter, Incremental Summary.

---

## 💻 System Requirements

| Spec | Minimum (Dev) | Recommended (Prod) |
| :--- | :--- | :--- |
| **CPU** | 2 vCPU | 4+ vCPU |
| **RAM** | 4GB | 16GB+ |
| **Disk** | 10GB SSD | 100GB NVMe |
| **OS** | Linux / macOS | Linux (K8s) |

---

## 🚀 Quick Start

### 1. Start Dependencies

```bash
docker-compose up -d
```

### 2. Configuration

Create `.env` file (optional) or set environment variables:
```bash
export GEMINI_API_KEY="your_key_here"
export QDRANT_API_KEY="your_qdrant_key"
```

Copy config file:
```bash
cp config.example.toml config.toml
# Edit config.toml to enable desired modules (Router, Wiki, etc.)
```

### 3. Run

```bash
# Default full-featured mode
cargo run --release --bin memoryos-gateway

# (Advanced) Enable specific features only (if Cargo.toml supports)
# cargo run --release --no-default-features --features "redis,qdrant"
```

### 4. Test

```bash
curl http://localhost:8080/health/status
```

**Detailed Guide**: [docs/QUICKSTART.md](./docs/QUICKSTART.md)

---

## 🏗️ Architecture

```mermaid
graph TD
    Client[User Client] -->|OpenAI Protocol| Gateway
    subgraph MemoryOS-Rust
        Gateway -->|Auth & Shield| Router{Smart Router}
        Router -->|Tier 0: FAQ| DirectHit[Direct Response]
        Router -->|Tier 1: Hot| LocalLLM[Local Llama]
        Router -->|Tier 2: Cold| CloudLLM[OpenAI/Gemini]
        Gateway -->|Async Event| Queue[NATS/Redis]
        Queue --> Worker
        Worker -->|Summarize| VectorDB[(Qdrant)]
        Worker -->|Export| Wiki[S3/Confluence]
    end
```

**Detailed Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)

---

## 📚 Documentation

### User Documentation
- [Quick Start](./docs/QUICKSTART.md) - Get started in 5 minutes
- [User Manual](./docs/USER_MANUAL.md) - Complete usage guide 📖
- [Architecture](./docs/ARCHITECTURE.md) - System design (Graph/Router)
- [API Reference](./docs/API.md) - API documentation
- [Development Guide](./docs/DEVELOPMENT.md) - Development setup
- [Deployment Guide](./docs/DEPLOYMENT.md) - K8s/Docker deployment
- [K3s Auto-Deploy](./docs/K3S_DEPLOYMENT.md) - One-click K8s cluster 🚀
- [Authentication](./docs/AUTH.md) - API Key management
- [FAQ System](./docs/FAQ_SYSTEM.md) - Auto-promote high-frequency Q&A ⚡

### Performance Optimization
- [Optimization Analysis](./docs/OPTIMIZATION.md) - Algorithm optimization strategies 🚀
- [Usage Guide](./docs/OPTIMIZATION_USAGE.md) - How to use optimization modules ⚡

### Deep Dive
- [Design Principles](./docs/DESIGN.md) - Design philosophy & implementation ⭐
- [Comparison](./docs/COMPARISON.md) - vs Mem0 analysis ⭐

### Developer Documentation
- [Roadmap](./docs/ROADMAP.md) - v0.2.0 → v1.0.0 planning
- [API Key Auth](./docs/AUTH.md) - Enterprise auth system (Qdrant persistence) 🔒
- [Work Log](./WORK_LOG.md) - **Who's doing what, for collaboration** ⭐⭐⭐
- [Project State](./docs/state.json) - AI context recovery (machine-readable)
- [Changelog](./CHANGELOG.md) - Version history
- [Contributing](./CONTRIBUTING.md) - Contribution guidelines
- [Documentation Index](./docs/README.md) - Complete docs navigation

**⭐ Recommended**: Design Principles and Comparison for system design insights

---

## 📊 Project Status

**Version**: 0.2.0  
**Status**: ✅ Production Ready  
**Completion**: 100%  

| Phase | Module | Status |
|-------|--------|--------|
| Phase 1 | Foundation (Config/Log) | ✅ |
| Phase 2 | Gateway & Adapters | ✅ |
| Phase 3 | Storage (Redis/Qdrant) | ✅ |
| Phase 4 | Intelligence (Router/Shield) | ✅ |
| Phase 5 | Worker & Async | ✅ |
| Phase 6 | Wiki Export | ✅ |
| Phase 7 | Graph Memory | ✅ |

---

## 🛠️ Tech Stack

- **Language**: Rust 1.93+
- **Async Runtime**: Tokio
- **Web Framework**: Axum
- **Short-term Storage**: Redis
- **Vector Storage**: Qdrant
- **LLM**: OpenAI, Gemini, Claude, Ollama, DeepSeek, OpenRouter, Azure

---

## 🤝 Contributing

Contributions are welcome! Please follow this workflow:

### Before Starting
1. 📖 Read [Development Guide](./docs/DEVELOPMENT.md)
2. 📝 Log your task in [WORK_LOG.md](./WORK_LOG.md)
3. 🔄 Pull latest code: `git pull`

### During Work
1. 📊 Update progress in [WORK_LOG.md](./WORK_LOG.md) daily
2. 🐛 Log issues immediately
3. 🔴 Update status if blocked

### After Completion
1. ✅ Mark task as complete in [WORK_LOG.md](./WORK_LOG.md)
2. 📝 Update [CHANGELOG.md](./CHANGELOG.md)
3. 🚀 Submit code: `git commit && git push`

**Collaboration**: We use `WORK_LOG.md` (human) + `docs/state.json` (AI) dual-track recording for transparent collaboration.

**Detailed Guide**: [CONTRIBUTING.md](./CONTRIBUTING.md)

---

## 🔧 Maintenance Status

**Current Status**: ✅ Production Ready & Actively Maintained

This project is **feature-complete** (100%) and in maintenance mode. We focus on:
- 🐛 Bug fixes and security updates
- 📚 Documentation improvements
- 💡 Community-driven enhancements

**See**: [MAINTENANCE.md](./MAINTENANCE.md) for detailed maintenance plan

---

## 📞 Contact

- **GitHub Issues**: [Report Issues](https://github.com/TelivANT/memoryos-rust/issues)
- **GitHub Discussions**: [Join Discussions](https://github.com/TelivANT/memoryos-rust/discussions)
- **Email**: 246803628+TelivANT@users.noreply.github.com
- **Security Issues**: Please email with subject `[SECURITY]`

---

## 📄 License

Apache 2.0 License - See [LICENSE](./LICENSE)

---

## 🌟 Related Projects

- **Original Project**: [MemoryOS](https://github.com/BAI-LAB/MemoryOS) - Python implementation
- **Paper**: [Memory OS of AI Agent](https://arxiv.org/abs/2506.06326)

---

**Version**: 0.2.0 | **Updated**: 2026-02-18
