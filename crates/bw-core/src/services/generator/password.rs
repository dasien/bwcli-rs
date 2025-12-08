use super::errors::GeneratorError;
use rand::Rng;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERS: &str = "0123456789";
const SPECIAL: &str = "!@#$%^&*";

const MIN_LENGTH: usize = 5;
const MAX_LENGTH: usize = 128;

/// Password generation options
#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub include_lowercase: bool,
    pub include_uppercase: bool,
    pub include_numbers: bool,
    pub include_special: bool,
    pub min_lowercase: usize,
    pub min_uppercase: usize,
    pub min_numbers: usize,
    pub min_special: usize,
    pub exclude_chars: Option<String>,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 14,
            include_lowercase: true,
            include_uppercase: true,
            include_numbers: true,
            include_special: true,
            min_lowercase: 0,
            min_uppercase: 0,
            min_numbers: 1,
            min_special: 1,
            exclude_chars: None,
        }
    }
}

/// Generate a cryptographically secure password with given options
///
/// # Arguments
/// * `options` - Password generation configuration
///
/// # Returns
/// Generated password string
///
/// # Errors
/// - Invalid options (constraints cannot be satisfied)
/// - RNG failure (should be extremely rare)
pub fn generate_password(options: &PasswordOptions) -> Result<String, GeneratorError> {
    validate_options(options)?;

    let mut rng = OsRng;

    // Build character sets
    let mut available_chars = String::new();
    let mut char_sets: Vec<(&str, usize)> = Vec::new();

    if options.include_lowercase {
        let chars = filter_excluded(LOWERCASE, &options.exclude_chars);
        if !chars.is_empty() {
            char_sets.push((LOWERCASE, options.min_lowercase));
            available_chars.push_str(&chars);
        }
    }
    if options.include_uppercase {
        let chars = filter_excluded(UPPERCASE, &options.exclude_chars);
        if !chars.is_empty() {
            char_sets.push((UPPERCASE, options.min_uppercase));
            available_chars.push_str(&chars);
        }
    }
    if options.include_numbers {
        let chars = filter_excluded(NUMBERS, &options.exclude_chars);
        if !chars.is_empty() {
            char_sets.push((NUMBERS, options.min_numbers));
            available_chars.push_str(&chars);
        }
    }
    if options.include_special {
        let chars = filter_excluded(SPECIAL, &options.exclude_chars);
        if !chars.is_empty() {
            char_sets.push((SPECIAL, options.min_special));
            available_chars.push_str(&chars);
        }
    }

    if available_chars.is_empty() {
        return Err(GeneratorError::NoCharacterSets);
    }

    // Filter available chars
    let available_chars = filter_excluded(&available_chars, &options.exclude_chars);
    let available_chars: Vec<char> = available_chars.chars().collect();

    // Start with minimum required characters
    let mut password_chars: Vec<char> = Vec::with_capacity(options.length);

    for (charset_str, min_count) in &char_sets {
        let filtered_charset = filter_excluded(charset_str, &options.exclude_chars);
        let charset: Vec<char> = filtered_charset.chars().collect();

        for _ in 0..*min_count {
            if password_chars.len() >= options.length {
                break;
            }
            let idx = rng.gen_range(0..charset.len());
            password_chars.push(charset[idx]);
        }
    }

    // Fill remaining with random characters from all available sets
    while password_chars.len() < options.length {
        let idx = rng.gen_range(0..available_chars.len());
        password_chars.push(available_chars[idx]);
    }

    // Shuffle using Fisher-Yates algorithm with OsRng
    password_chars.shuffle(&mut rng);

    Ok(password_chars.into_iter().collect())
}

fn validate_options(options: &PasswordOptions) -> Result<(), GeneratorError> {
    // Validate length
    if options.length < MIN_LENGTH || options.length > MAX_LENGTH {
        return Err(GeneratorError::InvalidLength(options.length));
    }

    // Check at least one set is enabled
    if !options.include_lowercase
        && !options.include_uppercase
        && !options.include_numbers
        && !options.include_special
    {
        return Err(GeneratorError::NoCharacterSets);
    }

    // Check minimum requirements don't exceed length
    let total_min =
        options.min_lowercase + options.min_uppercase + options.min_numbers + options.min_special;
    if total_min > options.length {
        return Err(GeneratorError::RequirementsExceedLength(
            total_min,
            options.length,
        ));
    }

    Ok(())
}

fn filter_excluded(chars: &str, exclude: &Option<String>) -> String {
    match exclude {
        None => chars.to_string(),
        Some(excluded) => chars.chars().filter(|c| !excluded.contains(*c)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_password_generation() {
        let options = PasswordOptions::default();
        let password = generate_password(&options).unwrap();

        assert_eq!(password.len(), 14);
        // Should contain at least 1 number and 1 special char
        assert!(password.chars().any(|c| NUMBERS.contains(c)));
        assert!(password.chars().any(|c| SPECIAL.contains(c)));
    }

    #[test]
    fn test_custom_length() {
        let mut options = PasswordOptions::default();
        options.length = 20;

        let password = generate_password(&options).unwrap();
        assert_eq!(password.len(), 20);
    }

    #[test]
    fn test_minimum_requirements() {
        let mut options = PasswordOptions::default();
        options.length = 10;
        options.min_lowercase = 2;
        options.min_uppercase = 2;
        options.min_numbers = 2;
        options.min_special = 2;

        let password = generate_password(&options).unwrap();
        assert_eq!(password.len(), 10);

        let lowercase_count = password.chars().filter(|c| LOWERCASE.contains(*c)).count();
        let uppercase_count = password.chars().filter(|c| UPPERCASE.contains(*c)).count();
        let number_count = password.chars().filter(|c| NUMBERS.contains(*c)).count();
        let special_count = password.chars().filter(|c| SPECIAL.contains(*c)).count();

        assert!(lowercase_count >= 2);
        assert!(uppercase_count >= 2);
        assert!(number_count >= 2);
        assert!(special_count >= 2);
    }

    #[test]
    fn test_excluded_characters() {
        let mut options = PasswordOptions::default();
        options.exclude_chars = Some("il1Lo0O".to_string());

        let password = generate_password(&options).unwrap();

        // Should not contain any excluded characters
        assert!(!password.contains('i'));
        assert!(!password.contains('l'));
        assert!(!password.contains('1'));
        assert!(!password.contains('L'));
        assert!(!password.contains('o'));
        assert!(!password.contains('0'));
        assert!(!password.contains('O'));
    }

    #[test]
    fn test_validation_invalid_length() {
        let mut options = PasswordOptions::default();
        options.length = 3;

        let result = generate_password(&options);
        assert!(matches!(result, Err(GeneratorError::InvalidLength(_))));
    }

    #[test]
    fn test_validation_requirements_exceed_length() {
        let mut options = PasswordOptions::default();
        options.length = 10;
        options.min_lowercase = 5;
        options.min_uppercase = 3;
        options.min_numbers = 2;
        options.min_special = 2; // Total = 12 > 10

        let result = generate_password(&options);
        assert!(matches!(
            result,
            Err(GeneratorError::RequirementsExceedLength(_, _))
        ));
    }

    #[test]
    fn test_validation_no_character_sets() {
        let mut options = PasswordOptions::default();
        options.include_lowercase = false;
        options.include_uppercase = false;
        options.include_numbers = false;
        options.include_special = false;

        let result = generate_password(&options);
        assert!(matches!(result, Err(GeneratorError::NoCharacterSets)));
    }

    #[test]
    fn test_only_numbers() {
        let mut options = PasswordOptions::default();
        options.include_lowercase = false;
        options.include_uppercase = false;
        options.include_special = false;
        options.min_numbers = 0;

        let password = generate_password(&options).unwrap();

        // All characters should be numbers
        assert!(password.chars().all(|c| NUMBERS.contains(c)));
    }
}
