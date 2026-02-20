# Project Status Dashboard

| Metric | Value |
| :--- | :--- |
| **Version** | v0.3.0 |
| **Build Status** | Passing |
| **Overall Completion** | ~78% |
| **Documentation** | Aligned with code |
| **Security Audit** | Passed (Internal) |

## Component Health

| Component | Status | Notes |
| :--- | :--- | :--- |
| **Gateway** | 🟢 Stable | FAQ API routes wired |
| **Worker** | 🟢 Stable | Auto-restart enabled |
| **Redis** | 🟢 Stable | STM coordination layer |
| **Qdrant** | 🟢 Stable | Vector search active |
| **Router** | 🟢 Active | Tier 0 FAQ direct hit implemented |
| **Wiki Export** | 🟢 Active | Local + S3 + Confluence backends |
| **FAQ System** | 🟢 Active | HeatTracker + AutoPromoter + Management API |

## v0.3.0 Features (completed 2026-02-20)
- Router Tier 0: FAQ direct hit bypasses LLM
- Wiki S3 export via OpenDAL S3ExportBackend
- Wiki Confluence export via REST API ConfluenceExportBackend
- FAQ management API (candidates, promote, delete, history, stats)
- Duplicate wiki exporter cleanup (core/wiki delegates to core/faq)

## Recent Activity
- **2026-02-20**: Released v0.3.0. FAQ router integration + wiki export backends + FAQ management API.
- **2026-02-20**: Docs aligned with code, ROADMAP updated.
- **2026-02-18**: Released v0.2.0-alpha. MVP with 3-Tier memory, 10 LLM adapters, security shield.
