//! User confirmation prompts for destructive operations

use super::VaultError;
use std::io::{self, Write};

/// Service for handling user confirmation prompts
pub struct ConfirmationService {
    no_interaction: bool,
}

impl ConfirmationService {
    /// Create new confirmation service
    pub fn new(no_interaction: bool) -> Self {
        Self { no_interaction }
    }

    /// Confirm permanent delete operation
    pub fn confirm_permanent_delete(&self) -> Result<bool, VaultError> {
        if self.no_interaction {
            return Ok(true); // Auto-confirm in non-interactive mode
        }

        self.prompt_yes_no("Are you sure you want to permanently delete this item? [y/N]: ")
    }

    /// Generic yes/no prompt
    fn prompt_yes_no(&self, message: &str) -> Result<bool, VaultError> {
        print!("{}", message);
        io::stdout()
            .flush()
            .map_err(|e| VaultError::IoError(e.to_string()))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| VaultError::IoError(e.to_string()))?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }
}
