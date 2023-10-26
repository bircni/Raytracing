[![Linux](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-linux.yml/badge.svg)](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-linux.yml)
[![Windows](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-windows.yml/badge.svg)](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-windows.yml)

# Raytracing

This is the main repository for the raytracing project.

## `log`

https://crates.io/crates/log

> Usage:  
> The basic use of the log crate is through the five logging macros: `error!`, `warn!`, `info!`, `debug!` and `trace!` where `error!` represents the highest-priority log messages and trace! the lowest. The log messages are filtered by configuring the log level to exclude messages with a lower priority. Each of these macros accept format strings similarly to println!.

## `anyhow`

https://crates.io/crates/anyhow

> Usage:  
> This library provides anyhow::Error, a trait object based error type for easy idiomatic error handling in Rust applications.

Example: 
```rust
use anyhow::Context;

fn test() -> anyhow::Result<()> {
    // watch out for the question mark operator

    let file = std::fs::File::create("test.txt").context("Failed to create test file")?;

    file.set_len(1234).context("Failed to set file length")?;

    Ok(())
}
```

If this fails the result is a fancy error message:
```
Error: Failed to create test file

Caused by:
    File exists (os error 17)
```