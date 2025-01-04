# THSR Ticket Booking Helper

This application is inspired by the design of [THSR-Ticket](https://github.com/BreezeWhite/THSR-Ticket) and has been modified and rewritten in Rust.
It is intended solely for coding practice and is provided as-is, without any guarantee or warranty.

## Prerequisites

The codes are written in Rust 1.83.0

## Run

```shell
# Checkout the repo
git clone https://github.com/Detoo/thsr-ticket-rs.git
cd thsr-ticket-rs

# Build
cargo build --release
```

### Option #1: To run and select presets interactively or input parameters manually
```shell
target/release/thsr-ticket-rs
```

### Option #2: To run with presets
1. Rename the file `.db/presets.json.template` to `.db/presets.json`
2. Modify `.db/presets.json` accordingly. Unfortunately the field names are not very human-readable. Only change it if you're sure about its effect
```shell
# Run with preset #1
target/release/thsr-ticket-rs -p 1
```
