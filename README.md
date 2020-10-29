# cargo-b64encode

[![CI](https://github.com/qryxip/cargo-b64encode/workflows/CI/badge.svg)](https://github.com/qryxip/cargo-b64encode/actions?workflow=CI)
[![dependency status](https://deps.rs/repo/github/qryxip/cargo-b64encode/status.svg)](https://deps.rs/repo/github/qryxip/cargo-b64encode)
[![Crates.io](https://img.shields.io/crates/v/cargo-b64encode.svg)](https://crates.io/crates/cargo-b64encode)
[![Crates.io](https://img.shields.io/crates/l/cargo-b64encode.svg)](https://crates.io/crates/cargo-b64encode)

A Cargo subcommand for creating standalone source files with base64-encoded payload.

## Installation

### Crates.io

```console
$ cargo install cargo-b64encode
```

### `master`

```console
$ cargo install --git https://github.com/qryxip/cargo-b64encode
```

### GitHub Releases

[Releases](https://github.com/qryxip/cargo-b64encode/releases)

## Usage

```console
$ cargo b64encode -o ./submission.rs
```

With [cargo-compete](https://crates.io/crates/cargo-compete),

```toml
[submit.transpile]
kind = "command"
args = ["cargo", "b64encode", "--bin", "{{ bin_name }}"]
```
