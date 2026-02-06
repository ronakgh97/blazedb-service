# 🔥 Blaze Service Layer

> **SaaS Platform for [BlazeDB](https://github.com/ronakgh97/blaze-db)** - Managed Vector Database as a Service

Blaze Service is the Backend/Proxy layer on top of BlazeDB as a SaaS platform, providing user
authentication, billing management, and instance provisioning for the high-performance vector database.

[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/) [![Docker](https://img.shields.io/badge/docker-20.10%2B-blue.svg)](https://www.docker.com/)

## Overview

Blaze Service handles the complete lifecycle of BlazeDB instances for SaaS customers:

- **User Authentication** with email verification (OTP-based)
- **Plan Management** (Free, Starter, Pro)
- **Instance Provisioning** for BlazeDB databases
- **Billing Integration** (Razorpay ready)
- **API Key Management** for secure access

## Key Features

### ✅ Implemented

- User Registration & Email Verification
- Secure OTP-based Authentication (PBKDF2-SHA256)
- API Key Generation & Hashing
- Email Service with HTML/Plain text support
- Custom JSON-based DataStore (persistent K/V storage)
- Automatic OTP cleanup (5-minute expiration)
- RESTful API with Axum
- Multi-tier Storage (data, logs, billing)
- Cloudflare Proxy Integration (SSL termination & forwarding)
- Docker Deployment (Dockerfile + docker-compose.yml) (Service only, no Proxy yet)
- BlazeDB Instance Management (provisioning, scaling, isolation) `BASIC IMPLEMENTATION`

### 🚧 Coming Soon

- Razorpay Billing Integration
- Cloudflare API & Tunnel Integration
- API Key Rotation & Revocation
- Embedding API Access
- Enhanced Monitoring & Logging
- Rate Limiting & Throttling
- Database Backup & Restore
- Plan & Quota Enforcement
- Admin Dashboard for User & Instance Management
- Better error handling and validation
- Some Edge cases in verification and instance provisioning

## 💾 Storage Engine (No external DBs)

Blaze Service uses a custom-built In-memory JSON-based key-value storage engine with:

- **ACID-like guarantees** via locking
- **Memory-mapped I/O** for performance
- **Atomic writes** with backup recovery
- **Type-safe operations** with generics

See [Storage engine Impl](src/server/storage.rs) for details.

Anyway, Not gonna lie, it's super hard to build a proper storage engine even the basic features, so I might switch to
SQLite
or RocksDB later 😔

## 📋 Subscription Plans

| Plan                  | Price/Month | Databases | Vectors/DB | Features                                                                                                               |
|-----------------------|-------------|-----------|------------|------------------------------------------------------------------------------------------------------------------------|
| **Free**              | $0          | 5         | 5K         | Dedicated User Container (CPU: 0.5 core, RAM: 512MB) + Any Dimension                                                   |
| **Starter**           | $9          | 10        | 100K       | Dedicated User Container (CPU: 3 core, RAM: 2GB) + Any Dimension + Priority Support + Backups                          |
| **Pro** (Coming Soon) | $29         | 20        | 500K       | Dedicated User AWS Instance + Any Dimension + Example Amazon Demo Dataset + Priority Support + Backups + Embedding API |

- All Plans included Demo Dataset (Amazon product 2023 embeddings)
- Support any dimension (Tested upto 1024D), but performance may degrade with higher dimensions

## 🔐 Security

- **OTP Hashing:** PBKDF2-HMAC-SHA256 (600,000 iterations)
- **Email Verification:** 6-digit codes with 1-minute expiration
- **API Keys:** Secure random generation + SHA-256 hashing
- **One-time Key Display:** API keys shown only once upon verification
- **Data Isolation:** Per-user instance segregation

## 🛠️ Technology Stack

- **Framework:** [Axum](https://github.com/tokio-rs/axum) (async backend framework)
- **Runtime:** [Tokio](https://tokio.rs/) (async runtime)
- **Email:** [Lettre](https://github.com/lettre/lettre) (SMTP client) (Maybe switch to SendGrid?) 😒
- **Crypto:** `sha2`, `pbkdf2`, `hex` - SHA-256 hashing for API keys & OTPs
- **Serialization:** `serde`, `serde_json`
- **Storage:** Custom JSON K/V store with `memmap2` (Maybe later switch to SQLite or RocksDB?) 😔

## 🤝 Contributing

Sure I guess...

## 🔗 Related Projects

- **[BlazeDB](https://github.com/ronakgh97/blaze-db)** - The core vector database engine
- Blaze Service (this repo) - SaaS platform & API

## 📞 Support

- **Issues:** [GitHub Issues](https://github.com/ronakgh97/blaze-service/issues)
- **Email:** noreply.blz.service@gmail.com

## 💡 Why Blaze Service?

Blaze Service makes it effortless to deploy and manage BlazeDB instances:

- **Zero Configuration:** Just register, verify and get your instance
- **Scalable:** Automatic scaling based on your plan
- **Secure:** Industry-standard API key hashing + Email verification
- **Affordable:** Free Forever tier available, pay as you grow
- **Good Performance:** Not gonna lie bro, checkout the benchmarks on BlazeDB repo

---

**Built with 🦀 by the BlazeDB Tea-..uh no...no Team, just me 🥲**
