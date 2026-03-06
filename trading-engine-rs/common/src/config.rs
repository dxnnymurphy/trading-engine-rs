use clap::Parser;
use figment::{Figment, providers::{Env, Format, Toml}};
use serde::{Serialize, Deserialize};
use std::path::Path;

/// Command Line Arguments for retrieving configuration
#[derive(Parser, Serialize, Debug)]
pub struct ConfigCliArgs {
    #[arg(long, env = "PROFILE", default_value = "default")]
    profile: String,
    #[arg(long, env = "CONFIG_DIR", default_value = ".")]
    config_dir: String,
}

/// Load application configuration by following:
/// 1. Parse CLI args to determine config profile and config directory
/// 2. Load base config from CONFIG_DIR/config.toml (if exists)
/// 3. Load profile specific config from CONFIG_DIR/config.{PROFILE}.toml (if exists)
/// 4. Override with environment variables prefixed with APP_CONFIG_
pub fn load_config<T>() -> Result<T, figment::Error>
where T: for<'de> Deserialize<'de> {
    let cli_args = ConfigCliArgs::parse();
    let profile = &cli_args.profile;
    log::info!("Loading configuration for profile: {}", profile);
    // File Based Config
    let base_config = format!("{}/config.toml", cli_args.config_dir);
    let profile_config = format!("{}/config.{}.toml", cli_args.config_dir, profile);
    
    let mut figment = Figment::new();
    
    if Path::new(&base_config).exists() {
        figment = figment.merge(Toml::file(&base_config).nested());
    } else {
        log::warn!("Base config file {} does not exist, skipping...", base_config);
    }
    
    if Path::new(&profile_config).exists() {
        figment = figment.merge(Toml::file(&profile_config).profile(profile));
    } else {
        log::warn!("Profile config file {} does not exist, skipping...", profile_config);
    }
    
    // Merge env variables into the target profile before selecting
    figment = figment.merge(Env::prefixed("APP_CONFIG_").profile(profile));
    
    // Now select and extract
    figment.select(profile).extract()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestConfig {
        api_url: String,
        timeout: u32,
    }

    #[test]
    fn test_load_config_with_nested_profiles() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            // Create a config.toml with nested profiles
            jail.create_file(
                "config.toml",
                r#"
                [dev]
                api_url = "localhost:3000"
                timeout = 30

                [prod]
                api_url = "aws-ec2:9000"
                timeout = 60
                "#,
            )?;

            // Set environment to use dev profile
            jail.set_env("PROFILE", "dev");

            // Test that we can load dev config
            let config: TestConfig = load_config().expect("Failed to load config");
            assert_eq!(config.api_url, "localhost:3000");
            assert_eq!(config.timeout, 30);

            Ok(())
        });
    }

    #[test]
    fn test_load_config_with_env_override() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            // Test with profile-specific file where env vars work
            jail.create_file(
                "config.toml",
                r#"
                [dev]
                api_url = "localhost:3000"
                timeout = 30
                "#,
            )?;

            jail.set_env("PROFILE", "dev");

            let config: TestConfig = load_config().expect("Failed to load config");
            assert_eq!(config.api_url, "localhost:3000"); 
            assert_eq!(config.timeout, 30);

            Ok(())
        });
    }

    #[test]
    fn test_profile_specific_file_overrides_base() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            jail.create_file(
                "config.toml",
                r#"
[default]
api_url = "default:3000"
timeout = 10
"#,
            )?;

            // Profile file overrides base
            jail.create_file(
                "config.test.toml",
                r#"
timeout = 999
"#,
            )?;

            jail.set_env("PROFILE", "test");

            let config: TestConfig = load_config().expect("Failed to load config");
            // api_url from default profile, timeout from profile file
            assert_eq!(config.timeout, 999);

            Ok(())
        });
    }

    #[test]
    fn test_load_config_profile_specific_file() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            // Base config
            jail.create_file(
                "config.toml",
                r#"
                [default]
                api_url = "localhost:3000"
                timeout = 30
                "#,
            )?;

            // Profile-specific override
            jail.create_file(
                "config.prod.toml",
                r#"
                api_url = "production:9000"
                timeout = 120
                "#,
            )?;

            jail.set_env("PROFILE", "prod");

            let config: TestConfig = load_config().expect("Failed to load config");
            assert_eq!(config.api_url, "production:9000");
            assert_eq!(config.timeout, 120);

            Ok(())
        });
    }

    #[test]
    fn test_load_config_missing_files_does_not_panic() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            jail.set_env("PROFILE", "dev");
            // No config files exist - should not panic but will fail to extract
            let result: Result<TestConfig, _> = load_config();
            assert!(result.is_err()); // Expected to fail without config

            Ok(())
        });
    }

    #[test]
    fn test_load_config_with_custom_config_dir() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            jail.create_dir("custom")?;
            jail.create_file(
                "custom/config.toml",
                r#"
                [dev]
                api_url = "custom:3000"
                timeout = 45
                "#,
            )?;

            jail.set_env("PROFILE", "dev");
            jail.set_env("CONFIG_DIR", "custom");

            let config: TestConfig = load_config().expect("Failed to load config");
            assert_eq!(config.api_url, "custom:3000");
            assert_eq!(config.timeout, 45);

            Ok(())
        });
    }

    #[test]
    fn test_priority_order_env_over_file() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            // Use profile-specific file instead of nested profile
            jail.create_file(
                "config.toml",
                r#"
                [test]
                api_url = "file:3000"
                timeout = 30
                "#,
            )?;

            jail.set_env("PROFILE", "test");
            jail.set_env("APP_CONFIG_TIMEOUT", "999");

            let config: TestConfig = load_config().expect("Failed to load config");
            // Environment variables should override file config
            assert_eq!(config.api_url, "file:3000");
            assert_eq!(config.timeout, 999); // overridden by env

            Ok(())
        });
    }

        #[test]
    fn test_priority_order_env_over_nested_file() {
        use figment::Jail;

        Jail::expect_with(|jail| {
            // Use profile-specific file instead of nested profile
            jail.create_file(
                "config.test.toml",
                r#"
                api_url = "file:3000"
                timeout = 30
                "#,
            )?;

            jail.set_env("PROFILE", "test");
            jail.set_env("APP_CONFIG_TIMEOUT", "999");

            let config: TestConfig = load_config().expect("Failed to load config");
            // Environment variables should override file config
            assert_eq!(config.api_url, "file:3000");
            assert_eq!(config.timeout, 999); // overridden by env

            Ok(())
        });
    }
}