# cargo-executable-payload

[![CI](https://github.com/qryxip/cargo-executable-payload/workflows/CI/badge.svg)](https://github.com/qryxip/cargo-executable-payload/actions?workflow=CI)
[![dependency status](https://deps.rs/repo/github/qryxip/cargo-executable-payload/status.svg)](https://deps.rs/repo/github/qryxip/cargo-executable-payload)
[![Crates.io](https://img.shields.io/crates/v/cargo-executable-payload.svg)](https://crates.io/crates/cargo-executable-payload)
[![Crates.io](https://img.shields.io/crates/l/cargo-executable-payload.svg)](https://crates.io/crates/cargo-executable-payload)

A Cargo subcommand for creating standalone source files with base64-encoded payload.

## Installation

### Crates.io

```console
$ cargo install cargo-executable-payload
```

### `master`

```console
$ cargo install --git https://github.com/qryxip/cargo-executable-payload
```

### GitHub Releases

[Releases](https://github.com/qryxip/cargo-executable-payload/releases)

## Usage

```console
$ cargo executable-payload -o ./submission.rs
```

With [cargo-compete](https://crates.io/crates/cargo-compete),

```toml
[submit.transpile]
kind = "command"
args = ["cargo", "executable-payload", "--bin", "{{ bin_name }}"]
```
