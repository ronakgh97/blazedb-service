# 🔥 Blaze Service

> **SaaS Platform for [BlazeDB](https://github.com/ronakgh97/blaze-db)** - Managed Vector Database as a Service

Blaze Service is the backend infrastructure that powers BlazeDB as a SaaS platform, providing user authentication,
billing management, and instance provisioning for the high-performance vector database.

[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org/)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

## Overview

Blaze Service handles the complete lifecycle of BlazeDB instances for SaaS customers:

- **User Authentication** with email verification (OTP-based)
- **Plan Management** (Free, Starter, Pro, Enterprise)
- **Instance Provisioning** for BlazeDB databases
- **Billing Integration** (Stripe ready)
- **API Key Management** for secure access

## Key Features

### ✅ Implemented

- User Registration & Email Verification
- Secure OTP-based Authentication (PBKDF2-SHA256)
- Email Service with HTML/Plain text support
- Custom JSON-based DataStore (persistent K/V storage)
- Automatic OTP cleanup (5-minute expiration)
- RESTful API with Axum
- Multi-tier Storage (data, logs, billing)

### 🚧 Coming Soon

- Stripe Billing Integration
- API Key Generation & Rotation
- BlazeDB Instance Provisioning
- Usage Tracking & Quotas
- Database Backup & Restore

## 💾 Storage Engine (Mhm, Databases are boring 😔)

Blaze Service uses a custom-built JSON-based key-value storage engine with:

- **ACID-like guarantees** via file locking
- **Memory-mapped I/O** for performance
- **Atomic writes** with backup recovery
- **Type-safe operations** with generics

See [Storage engine Impl](src/server/storage.rs) for details.

## 📋 Subscription Plans

| Plan        | Price/Month | Databases | Vectors/DB | Features                                                                               |
|-------------|-------------|-----------|------------|----------------------------------------------------------------------------------------|
| **Free**    | $0          | 2         | 10K        | Shared Instance + Any Dimensions + Example Amazon Demo Dataset + Limited API Endpoints |
| **Starter** | $12         | 10        | 100K       | Dedicated Instance + Any Dimensions + Example Amazon Demo Dataset                      |
| **Pro**     | $29         | 100       | 1M         | Dedicated Instance + Any Dimensions + Example Amazon Demo Dataset + Embedding API      |

## 🔐 Security

- **OTP Hashing:** PBKDF2-HMAC-SHA256 (600,000 iterations)
- **Email Verification:** 6-digit codes with 5-minute expiration
- **API Keys:** Secure random generation (coming soon)
- **Data Isolation:** Per-user instance segregation

## 🛠️ Technology Stack

- **Framework:** [Axum](https://github.com/tokio-rs/axum) (async web framework)
- **Runtime:** [Tokio](https://tokio.rs/) (async runtime)
- **Email:** [Lettre](https://github.com/lettre/lettre) (SMTP client) (Maybe switch to SendGrid?) 😒
- **Crypto:** `sha2`, `pbkdf2`, `hex`
- **Serialization:** `serde`, `serde_json`
- **Storage:** Custom JSON K/V store with `memmap2`

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
- **Secure:** Email verification and API key authentication
- **Affordable:** Free tier available, pay as you grow
- **Good Performance:** Not gonna lie bro, checkout the benchmarks on BlazeDB repo

---

**Built with 🦀 by the BlazeDB Tea-..uh no...no Team, actually just me**
