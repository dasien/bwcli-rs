//! CSV export formatter

use crate::models::vault::{CipherType, CipherView};
use crate::services::import_export::errors::ExportError;
use crate::services::import_export::export::{ExportData, ExportFormatter, ExportOptions};
use async_trait::async_trait;
use csv::WriterBuilder;

/// CSV export formatter
pub struct CsvFormatter;

impl CsvFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Convert cipher to CSV record
    /// Returns a record with exactly 34 columns to match the Bitwarden CSV format
    fn cipher_to_record(&self, cipher: &CipherView, folder_name: &str) -> Vec<String> {
        let mut record = vec![
            folder_name.to_string(),
            if cipher.favorite { "1" } else { "0" }.to_string(),
            Self::type_to_string(cipher.cipher_type),
            cipher.name.clone(),
            cipher.notes.clone().unwrap_or_default(),
        ];

        // Custom fields
        let fields_str = cipher
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.name, f.value.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");
        record.push(fields_str);

        // Reprompt (always 0 for now)
        record.push("0".to_string());

        // Login fields (columns 7-10: 4 columns)
        match cipher.cipher_type {
            CipherType::Login => {
                if let Some(login) = &cipher.login {
                    let uris = login
                        .uris
                        .iter()
                        .filter_map(|u| u.uri.clone())
                        .collect::<Vec<_>>()
                        .join("\n");
                    record.push(uris);
                    record.push(login.username.clone().unwrap_or_default());
                    record.push(login.password.clone().unwrap_or_default());
                    record.push(login.totp.clone().unwrap_or_default());
                } else {
                    record.extend(vec![String::new(); 4]);
                }
            }
            _ => {
                // Empty login fields for non-login types
                record.extend(vec![String::new(); 4]);
            }
        }

        // Card fields (columns 11-16: 6 columns)
        match cipher.cipher_type {
            CipherType::Card => {
                if let Some(card) = &cipher.card {
                    record.push(card.cardholder_name.clone().unwrap_or_default());
                    record.push(card.brand.clone().unwrap_or_default());
                    record.push(card.number.clone().unwrap_or_default());
                    record.push(card.exp_month.clone().unwrap_or_default());
                    record.push(card.exp_year.clone().unwrap_or_default());
                    record.push(card.code.clone().unwrap_or_default());
                } else {
                    record.extend(vec![String::new(); 6]);
                }
            }
            _ => {
                // Empty card fields for non-card types
                record.extend(vec![String::new(); 6]);
            }
        }

        // Identity fields (columns 17-33: 17 columns)
        // Note: The spec includes 'company' but the model doesn't have it yet
        match cipher.cipher_type {
            CipherType::Identity => {
                if let Some(identity) = &cipher.identity {
                    record.push(identity.title.clone().unwrap_or_default());
                    record.push(identity.first_name.clone().unwrap_or_default());
                    record.push(identity.middle_name.clone().unwrap_or_default());
                    record.push(identity.last_name.clone().unwrap_or_default());
                    record.push(identity.address1.clone().unwrap_or_default());
                    record.push(identity.address2.clone().unwrap_or_default());
                    record.push(identity.address3.clone().unwrap_or_default());
                    record.push(identity.city.clone().unwrap_or_default());
                    record.push(identity.state.clone().unwrap_or_default());
                    record.push(identity.postal_code.clone().unwrap_or_default());
                    record.push(identity.country.clone().unwrap_or_default());
                    record.push(identity.email.clone().unwrap_or_default());
                    record.push(identity.phone.clone().unwrap_or_default());
                    record.push(identity.ssn.clone().unwrap_or_default());
                    record.push(identity.username.clone().unwrap_or_default());
                    record.push(identity.passport_number.clone().unwrap_or_default());
                    record.push(identity.license_number.clone().unwrap_or_default());
                } else {
                    record.extend(vec![String::new(); 17]);
                }
            }
            _ => {
                // Empty identity fields for non-identity types
                record.extend(vec![String::new(); 17]);
            }
        }

        record
    }

    fn type_to_string(cipher_type: CipherType) -> String {
        match cipher_type {
            CipherType::Login => "login".to_string(),
            CipherType::SecureNote => "note".to_string(),
            CipherType::Card => "card".to_string(),
            CipherType::Identity => "identity".to_string(),
            CipherType::SshKey => "sshkey".to_string(),
        }
    }
}

#[async_trait]
impl ExportFormatter for CsvFormatter {
    fn format_name(&self) -> &str {
        "csv"
    }

    fn file_extension(&self) -> &str {
        "csv"
    }

    async fn format(
        &self,
        data: &ExportData,
        _options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError> {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);

        // Write header with all columns for all item types
        wtr.write_record(&[
            "folder",
            "favorite",
            "type",
            "name",
            "notes",
            "fields",
            "reprompt",
            // Login fields
            "login_uri",
            "login_username",
            "login_password",
            "login_totp",
            // Card fields
            "card_cardholderName",
            "card_brand",
            "card_number",
            "card_expMonth",
            "card_expYear",
            "card_code",
            // Identity fields (missing 'company' - model doesn't have it)
            "identity_title",
            "identity_firstName",
            "identity_middleName",
            "identity_lastName",
            "identity_address1",
            "identity_address2",
            "identity_address3",
            "identity_city",
            "identity_state",
            "identity_postalCode",
            "identity_country",
            "identity_email",
            "identity_phone",
            "identity_ssn",
            "identity_username",
            "identity_passportNumber",
            "identity_licenseNumber",
        ])?;

        // Build folder name map
        let folder_map: std::collections::HashMap<String, String> = data
            .folders
            .iter()
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();

        // Write ciphers
        for cipher in &data.ciphers {
            let folder_name = cipher
                .folder_id
                .as_ref()
                .and_then(|id| folder_map.get(id))
                .cloned()
                .unwrap_or_default();

            let record = self.cipher_to_record(cipher, &folder_name);
            wtr.write_record(&record)?;
        }

        wtr.flush()?;
        let inner = wtr
            .into_inner()
            .map_err(|e| ExportError::IoError(e.into_error()))?;
        Ok(inner)
    }

    fn requires_password(&self) -> bool {
        false
    }

    fn is_encrypted(&self) -> bool {
        false
    }
}
