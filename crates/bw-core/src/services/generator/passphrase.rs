use super::errors::GeneratorError;
use super::wordlist::get_wordlist;
use rand::Rng;
use rand::rngs::OsRng;

const MIN_WORDS: usize = 3;
const MAX_WORDS: usize = 20;

/// Passphrase generation options
#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    pub num_words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub include_number: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            num_words: 3,
            separator: "-".to_string(),
            capitalize: false,
            include_number: false,
        }
    }
}

/// Generate a cryptographically secure passphrase with given options
///
/// # Arguments
/// * `options` - Passphrase generation configuration
///
/// # Returns
/// Generated passphrase string
///
/// # Errors
/// - Invalid options (word count out of range)
/// - RNG failure (should be extremely rare)
pub fn generate_passphrase(options: &PassphraseOptions) -> Result<String, GeneratorError> {
    validate_options(options)?;

    let wordlist = get_wordlist();
    let mut rng = OsRng;

    // Select random words
    let mut words: Vec<String> = Vec::with_capacity(options.num_words);
    for _ in 0..options.num_words {
        let idx = rng.gen_range(0..wordlist.len());
        let mut word = wordlist[idx].to_string();

        if options.capitalize {
            // Capitalize first letter
            if let Some(first_char) = word.chars().next() {
                word = first_char
                    .to_uppercase()
                    .chain(word.chars().skip(1))
                    .collect();
            }
        }

        words.push(word);
    }

    // Join with separator
    let mut passphrase = words.join(&options.separator);

    // Optionally append a random number
    if options.include_number {
        let number = rng.gen_range(0..10000);
        passphrase.push_str(&options.separator);
        passphrase.push_str(&number.to_string());
    }

    Ok(passphrase)
}

fn validate_options(options: &PassphraseOptions) -> Result<(), GeneratorError> {
    if options.num_words < MIN_WORDS || options.num_words > MAX_WORDS {
        return Err(GeneratorError::InvalidWordCount(options.num_words));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_passphrase_generation() {
        let options = PassphraseOptions::default();
        let passphrase = generate_passphrase(&options).unwrap();

        // Should contain 3 words separated by hyphens
        let parts: Vec<&str> = passphrase.split('-').collect();
        assert_eq!(parts.len(), 3);

        // Words should not be capitalized by default
        assert!(
            parts
                .iter()
                .all(|word| { word.chars().next().map_or(false, |c| c.is_lowercase()) })
        );
    }

    #[test]
    fn test_custom_word_count() {
        let mut options = PassphraseOptions::default();
        options.num_words = 5;

        let passphrase = generate_passphrase(&options).unwrap();
        let parts: Vec<&str> = passphrase.split('-').collect();
        assert_eq!(parts.len(), 5);
    }

    #[test]
    fn test_custom_separator() {
        let mut options = PassphraseOptions::default();
        options.separator = ".".to_string();

        let passphrase = generate_passphrase(&options).unwrap();
        assert!(passphrase.contains('.'));

        let parts: Vec<&str> = passphrase.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_capitalization() {
        let mut options = PassphraseOptions::default();
        options.capitalize = true;

        let passphrase = generate_passphrase(&options).unwrap();
        let parts: Vec<&str> = passphrase.split('-').collect();

        // Each word should start with uppercase
        assert!(
            parts
                .iter()
                .all(|word| { word.chars().next().map_or(false, |c| c.is_uppercase()) })
        );
    }

    #[test]
    fn test_include_number() {
        let mut options = PassphraseOptions::default();
        options.include_number = true;

        let passphrase = generate_passphrase(&options).unwrap();
        let parts: Vec<&str> = passphrase.split('-').collect();

        // Should have 4 parts: 3 words + 1 number
        assert_eq!(parts.len(), 4);

        // Last part should be a number
        assert!(parts[3].chars().all(|c| c.is_numeric()));
    }

    #[test]
    fn test_validation_word_count_too_low() {
        let mut options = PassphraseOptions::default();
        options.num_words = 2;

        let result = generate_passphrase(&options);
        assert!(matches!(result, Err(GeneratorError::InvalidWordCount(_))));
    }

    #[test]
    fn test_validation_word_count_too_high() {
        let mut options = PassphraseOptions::default();
        options.num_words = 21;

        let result = generate_passphrase(&options);
        assert!(matches!(result, Err(GeneratorError::InvalidWordCount(_))));
    }

    #[test]
    fn test_passphrase_uses_valid_words() {
        let options = PassphraseOptions::default();
        let passphrase = generate_passphrase(&options).unwrap();

        let wordlist = get_wordlist();
        let parts: Vec<&str> = passphrase.split('-').collect();

        // Each part should be a valid word from the wordlist
        for part in parts {
            assert!(wordlist.contains(&part));
        }
    }

    #[test]
    fn test_passphrase_randomness() {
        let options = PassphraseOptions::default();

        // Generate multiple passphrases
        let mut passphrases = Vec::new();
        for _ in 0..10 {
            passphrases.push(generate_passphrase(&options).unwrap());
        }

        // They should not all be the same (extremely unlikely with 7776^3 possibilities)
        let first = &passphrases[0];
        assert!(passphrases.iter().any(|p| p != first));
    }
}
