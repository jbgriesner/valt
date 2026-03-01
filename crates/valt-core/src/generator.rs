use rand::Rng;

use crate::error::CoreError;

const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}|;:,.<>?";

/// Options for the password generator.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
        }
    }
}

/// Generate a random password from the given configuration.
pub fn generate(config: &GeneratorConfig) -> Result<String, CoreError> {
    if config.length == 0 {
        return Err(CoreError::InvalidLength);
    }

    let mut charset = String::new();
    if config.uppercase {
        charset.push_str(UPPERCASE);
    }
    if config.lowercase {
        charset.push_str(LOWERCASE);
    }
    if config.digits {
        charset.push_str(DIGITS);
    }
    if config.symbols {
        charset.push_str(SYMBOLS);
    }

    if charset.is_empty() {
        return Err(CoreError::EmptyCharset);
    }

    let chars: Vec<char> = charset.chars().collect();
    let mut rng = rand::thread_rng();
    let password: String = (0..config.length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();

    Ok(password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_respected() {
        let cfg = GeneratorConfig {
            length: 32,
            ..Default::default()
        };
        let pwd = generate(&cfg).unwrap();
        assert_eq!(pwd.len(), 32);
    }

    #[test]
    fn test_only_digits() {
        let cfg = GeneratorConfig {
            length: 100,
            uppercase: false,
            lowercase: false,
            digits: true,
            symbols: false,
        };
        let pwd = generate(&cfg).unwrap();
        assert!(pwd.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_only_uppercase() {
        let cfg = GeneratorConfig {
            length: 100,
            uppercase: true,
            lowercase: false,
            digits: false,
            symbols: false,
        };
        let pwd = generate(&cfg).unwrap();
        assert!(pwd.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn test_empty_charset_error() {
        let cfg = GeneratorConfig {
            length: 10,
            uppercase: false,
            lowercase: false,
            digits: false,
            symbols: false,
        };
        assert!(matches!(generate(&cfg), Err(CoreError::EmptyCharset)));
    }

    #[test]
    fn test_zero_length_error() {
        let cfg = GeneratorConfig {
            length: 0,
            ..Default::default()
        };
        assert!(matches!(generate(&cfg), Err(CoreError::InvalidLength)));
    }

    #[test]
    fn test_two_calls_differ() {
        let cfg = GeneratorConfig::default();
        let a = generate(&cfg).unwrap();
        let b = generate(&cfg).unwrap();
        // Statistically guaranteed to differ with 20 chars from a 90-char charset
        assert_ne!(a, b);
    }
}
