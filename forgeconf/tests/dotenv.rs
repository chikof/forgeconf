#![allow(dead_code)]

#[cfg(feature = "dotenv")]
mod tests {
    use forgeconf::{ConfigError, DotenvSource, forgeconf};

    #[forgeconf(config(path = "tests/fixtures/basic.toml"))]
    struct DotenvConfig {
        port: u16,
        database_url: String,
    }

    #[forgeconf]
    struct DotenvOnlyConfig {
        port: u16,
        database_url: String,
    }

    #[test]
    fn dotenv_overrides_file_values() -> Result<(), ConfigError> {
        let cfg = DotenvConfig::loader()
            .add_source(DotenvSource::new("tests/fixtures/test.env").with_priority(20))
            .load()?;
        assert_eq!(cfg.port, 9000);
        assert!(cfg.database_url.contains("dotenv"));
        Ok(())
    }

    #[test]
    fn dotenv_as_sole_source() -> Result<(), ConfigError> {
        let cfg = DotenvOnlyConfig::loader()
            .add_source(DotenvSource::new("tests/fixtures/test.env"))
            .load()?;
        assert_eq!(cfg.port, 9000);
        Ok(())
    }

    #[test]
    fn missing_file_silently_ignored() -> Result<(), ConfigError> {
        let cfg = DotenvConfig::loader()
            .add_source(DotenvSource::new("tests/fixtures/nonexistent.env"))
            .load()?;
        assert_eq!(cfg.port, 3000);
        Ok(())
    }

    #[test]
    fn required_missing_file_errors() {
        let result = DotenvOnlyConfig::loader()
            .add_source(DotenvSource::new("tests/fixtures/nonexistent.env").required())
            .load();
        assert!(result.is_err());
    }
}
