# foothold

A learning project climbing toward a perpetuals DEX on Solana — built rung by rung, the hard way.

Each rung is a standalone project that teaches the layer the next one needs. Code is written by hand; this repo is the paper trail.

---

## The ladder

### Rung 1 — TradFi trading backend (Rust + AWS) ← current

A full centralized-exchange backend, from scratch, in Rust, deployed to AWS. No blockchain yet — pure off-chain systems. Highest learning-per-hour, and it makes the on-chain DEX design read like plain English once you finish.

**Why start here?** Most DeFi engineers ship smart contracts without ever understanding what a real exchange actually does internally. This rung fixes that.

| Component | Status |
|-----------|--------|
| Matching engine core (order book, price-time priority, cancel) | ✅ |
| Sequencer — deterministic replay | 🔲 |
| Risk engine + Redis fund locking | 🔲 |
| API gateway — REST + WebSocket | 🔲 |
| Message bus + DB workers → Postgres | 🔲 |
| Market data service | 🔲 |
| Docker + local docker-compose | 🔲 |
| AWS deploy (ECS, RDS, ElastiCache, MSK) | 🔲 |
| Terraform IaC | 🔲 |
| CI/CD (GitHub Actions) | 🔲 |
| Load test → 100k TPS / <1ms matching | 🔲 |

### Rung 2 — Toy spot DEX on-chain (Anchor)

Minimal on-chain swap or order book. Where Solana's real teeth show: account model, PDAs, CPIs, rent, compute budget.

### Rung 3 — Perps (someday, devnet only)

Oracle integration, funding-rate math, margin accounting, liquidation engine. Never real capital until audited.

---

## Architecture (Rung 1)

Two pipelines:

```
Hot path (latency-critical, no DB writes)
  API Gateway → Risk Engine → Sequencer → Matching Engine

Cold path (async)
  Matching Engine → Message Bus → DB Workers → Postgres
                                → Market Data → WebSocket fans
```

**Matching engine** — pure in-memory, single-threaded per symbol, price-time priority. Bids sorted highest-first, asks lowest-first. O(1) cancel via an order index.

**Sequencer** — assigns a strictly increasing global sequence ID. Replay the log to rebuild state after a crash — the source of determinism.

**Risk engine** — pre-trade checks against Redis balances. Atomically locks funds before an order enters the sequencer.

**Target:** < 1ms matching internally, < 50ms end-to-end, ~100k orders/sec peak.

---

## Stack

| Layer | Choice |
|-------|--------|
| Language | Rust |
| Async runtime | tokio |
| HTTP / WebSocket | axum |
| Message bus | Kafka (rdkafka) / Redpanda |
| Database | Postgres via sqlx |
| Cache / locks | Redis |
| Money | rust_decimal (never f64) |
| IaC | Terraform |
| CI/CD | GitHub Actions |
| Cloud | AWS (ECS Fargate, RDS, ElastiCache, MSK, ECR) |

---

## Getting started

```bash
git clone https://github.com/davigiroux/foothold
cd foothold
cargo test -p matching-engine
```

Requires Rust stable. No other dependencies for the engine core.

---

## Design space

Two poles this project explores:

**Performance-first** — central sequencer, in-memory matching, off-chain order book + on-chain settlement. The TradFi model; roughly what Hyperliquid does.

**Composability-first** — fully on-chain, betting the chain evolves to close the performance gap. Phoenix's bet on Solana L1 (Alpenglow, BAM, MCL).

Understanding both ends = understanding the whole map.
