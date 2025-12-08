/// EFF Long Wordlist for passphrase generation
/// Source: https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt
///
/// This is a list of 7776 words optimized for passphrase generation.
/// Each word is unique, memorable, and easy to type.
const EFF_WORDLIST: &str = include_str!("eff_large_wordlist.txt");

/// Get the EFF wordlist as a vector of words
pub fn get_wordlist() -> Vec<&'static str> {
    EFF_WORDLIST.lines().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wordlist_size() {
        let words = get_wordlist();
        // EFF long wordlist has 7776 words
        assert_eq!(words.len(), 7776);
    }

    #[test]
    fn test_wordlist_contains_valid_words() {
        let words = get_wordlist();
        // Check some known words from the list (first and last words)
        assert!(words.iter().any(|&w| w == "abacus"));
        assert!(words.iter().any(|&w| w == "zoom"));
    }

    #[test]
    fn test_no_empty_words() {
        let words = get_wordlist();
        assert!(words.iter().all(|w| !w.is_empty()));
    }
}
