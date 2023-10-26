[![Linux](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-linux.yml/badge.svg)](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-linux.yml)
[![Windows](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-windows.yml/badge.svg)](https://github.com/TrayRacers/Raytracing/actions/workflows/rust-windows.yml)

# Raytracing

This is the main repository for the raytracing project.

## SimpleLog Usage

```rust
// error
error!("error message");

// warn
warn!("warn message");

// info (only appears in the log file)
info!("info message");

// debug
debug!("debug message");

// trace
trace!("trace message");
```