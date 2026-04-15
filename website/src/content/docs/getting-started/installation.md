---
title: Installation
description: How to install skillmark
---

## Requirements

- [Rust toolchain](https://rustup.rs/) (1.80 or later)

## Install from crates.io

```bash
cargo install skillmark
```

## Verify

```bash
skillmark --version
```

You should see `skillmark 0.1.0` (or the latest version).

## Build from source

```bash
git clone https://github.com/michellepellon/skillmark.git
cd skillmark
cargo install --path crates/skillmark
```
