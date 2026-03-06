# Common - Reusable Configuration and Logging

A reusable Rust library providing standardized configuration management and logging initialization for applications.

## Features

- **Hierarchical Configuration**: File < Environment Variables < CLI Arguments
- **Profile-Based Config**: Support for multiple environments (dev, staging, prod, etc.)
- **Type-Safe**: Generic configuration loading with compile-time type checking
- **Simple Logging**: Pre-configured `env_logger` with sensible defaults

---

## Configuration (`config` module)

### Overview

The `config` module provides a flexible configuration system using `figment` and `clap`:

1. **CLI Arguments** determine meta-configuration (profile, config directory)
2. **TOML Files** provide base and profile-specific settings
3. **Environment Variables** override file-based config

### Priority Order

```
config.toml < config.{profile}.toml < ENV variables (APP_CONFIG_*)
```

Higher priority sources override lower priority ones.

### Usage

**Define your application config:**

```rust
use common::config::load_config;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct MyAppConfig {
    api_url: String,
    timeout: u32,
    max_retries: u32,
}

fn main() {
    let config: MyAppConfig = load_config()
        .expect("Failed to load configuration");
    
    println!("API URL: {}", config.api_url);
}
```

**Create a `config.toml`:**

```toml
[dev]
api_url = "localhost:3000"
timeout = 30
max_retries = 3

[prod]
api_url = "api.example.com:443"
timeout = 60
max_retries = 5
```

**Run with different profiles:**

```bash
# Uses default profile
cargo run

# Uses dev profile
cargo run -- --profile dev

# Uses prod profile with custom config location
cargo run -- --profile prod --config-dir /etc/myapp

# Override via environment variable
APP_CONFIG_API_URL=custom:9000 cargo run -- --profile dev
```

### CLI Arguments

| Argument | Environment Variable | Default | Description |
|----------|---------------------|---------|-------------|
| `--profile` | `PROFILE` | `default` | Configuration profile to load |
| `--config-dir` | `CONFIG_DIR` | `.` | Base directory for config files |

### Environment Variables

Any configuration field can be overridden using environment variables with the `APP_CONFIG_` prefix:

```bash
APP_CONFIG_API_URL=example.com:8080
APP_CONFIG_TIMEOUT=120
APP_CONFIG_MAX_RETRIES=10
```

### File Structure

The config system looks for:
1. `{config_dir}/config.toml` - Base configuration with profiles
2. `{config_dir}/config.{profile}.toml` - Optional profile-specific overrides

Both files are optional and will log warnings if missing.

---

## Logging (`logging` module)

### Overview

Pre-configured `env_logger` with sensible defaults for quick setup.

### Usage

```rust
use common::logging::init_logging;

fn main() {
    init_logging();
    
    log::info!("Application started");
    log::debug!("Debug information");
}
```

### Default Behavior

- **Default Level**: `INFO` (shows info, warn, error)
- **Override**: Set `RUST_LOG` environment variable

### Examples

```bash
# Use default (INFO level)
cargo run

# Debug level for all modules
RUST_LOG=debug cargo run

# Info for app, debug for specific module
RUST_LOG=info,myapp::database=debug cargo run

# Only errors
RUST_LOG=error cargo run
```

### Log Levels

| Level | Description |
|-------|-------------|
| `error` | Critical errors only |
| `warn` | Warnings and errors |
| `info` | General information (default) |
| `debug` | Detailed debugging information |
| `trace` | Very verbose tracing |

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
common = { path = "./common" }
```

---

## Complete Example

```rust
use common::{config::load_config, logging::init_logging};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct AppConfig {
    api_url: String,
    timeout: u32,
}

fn main() {
    // Initialize logging first
    init_logging();
    
    // Load configuration
    let config: AppConfig = load_config()
        .expect("Failed to load configuration");
    
    log::info!("Starting application");
    log::info!("API URL: {}", config.api_url);
    log::debug!("Timeout: {}s", config.timeout);
    
    // Your application logic here
}
```

**config.toml:**
```toml
[default]
api_url = "localhost:3000"
timeout = 30

[dev]
api_url = "localhost:8000"

[prod]
api_url = "api.production.com"
timeout = 60
```

**Run:**
```bash
# Development
RUST_LOG=debug cargo run -- --profile dev

# Production
cargo run -- --profile prod
```

---

## Future Improvements

- **Current**: `env_logger` (simple, synchronous)
- **Upgrade Path**: Consider `tracing` for high-performance applications
  - Async logging
  - Structured logging
  - Better performance for high-throughput systems

---

## License

This library is intended for internal use across projects.
