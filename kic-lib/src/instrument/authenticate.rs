// Authenticate functionality of the instrument.

#[cfg(test)]
use keyring::{mock, set_default_credential_builder};

use crate::{model::Model, InstrumentError};

const SERVICE_NAME: &str = "tsp-toolkit";

/// An enum that provides the expected functionality for authentication into an instrument.
///
/// Some instruments may not have the concept of authentication in, for those instruments,
/// simply pass `Authentiate` struct to the instrument and do not use it in Login trait.
///
/// Note that none of the functions or implementing structures have built-in
/// cryptography and are therefore not inherently secure. Communication with most
/// instruments is also done in plain text and therefore it seems disingenuous to
/// focus on security here.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Authentication {
    /// Prompt the user on the commandline for their username (visible) and password (hidden).
    Prompt,
    PromptPartial {
        username: Option<String>,
        password: Option<String>,
    },
    /// Allows a credential to be entered without prompts or keyring lookups.
    Credential { username: String, password: String },
    /// Uses an id to look up the proper credentials in the system keyring.
    Keyring { id: String },
    /// No authentication is required, don't try to use any.
    NoAuth,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct SecretEntry {
    username: String,
    password: String,
}

impl Authentication {
    ///
    /// Retrieves the username
    ///
    /// # Errors
    ///
    /// Returns an error if the retrieval fails.
    ///
    pub fn read_username(&mut self) -> Result<Option<String>, InstrumentError> {
        match self {
            Self::Prompt => {
                eprintln!("Enter Username:");
                let mut username = String::new();
                std::io::stdin().read_line(&mut username)?;
                let username = username.trim();
                *self = Self::PromptPartial {
                    username: Some(username.to_string()),
                    password: None,
                };
                Ok(Some(username.to_string()))
            }
            Self::PromptPartial { username, .. } if username.is_some() => Ok(username.clone()),
            Self::PromptPartial { password, username } if username.is_none() => {
                let mut username = String::new();
                std::io::stdin().read_line(&mut username)?;
                let username = username.trim();
                *self = Self::PromptPartial {
                    username: Some(username.to_string()),
                    password: password.take(),
                };
                eprintln!("Enter Password (characters hidden):");
                Ok(Some(username.to_string()))
            }
            Self::PromptPartial { .. } => {
                unreachable!("All other partial prompt options are used for usernames")
            }
            Self::Credential { username, .. } => Ok(Some((*username).to_string())),
            Self::Keyring { id } => {
                let entry = keyring::Entry::new(SERVICE_NAME, id)?;
                let secret = &entry.get_secret()?;
                let secret: SecretEntry =
                    serde_json::from_str(String::from_utf8_lossy(secret).as_ref())?;

                Ok(Some(secret.username))
            }
            Self::NoAuth => Ok(None),
        }
    }

    /// Retrieves the password
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if any errors occurred.
    ///
    pub fn read_password(&mut self) -> Result<Option<String>, InstrumentError> {
        match self {
            Self::Prompt => {
                eprintln!("Enter Password (characters hidden):");
                let password = rpassword::read_password()?;
                *self = Self::PromptPartial {
                    username: None,
                    password: Some(password.clone()),
                };
                Ok(Some(password))
            }
            Self::PromptPartial { password, username } if password.is_none() => {
                eprintln!("Enter Password (characters hidden):");
                let password = rpassword::read_password()?;
                *self = Self::PromptPartial {
                    username: username.take(),
                    password: Some(password.clone()),
                };
                eprintln!("Enter Password (characters hidden):");
                Ok(Some(password))
            }
            Self::PromptPartial { password, .. } if password.is_some() => Ok(password.clone()),
            Self::PromptPartial { .. } => {
                unreachable!("All other prompt options are used for passwords")
            }
            Self::Credential { password, .. } => Ok(Some((*password).to_string())),
            Self::Keyring { id } => {
                let entry = keyring::Entry::new(SERVICE_NAME, id)?;
                let secret = &entry.get_secret()?;
                let secret: SecretEntry =
                    serde_json::from_str(String::from_utf8_lossy(secret).as_ref())?;

                Ok(Some(secret.password))
            }
            Self::NoAuth => Ok(None),
        }
    }

    /// Checks to see if this keyring entry exists
    ///
    /// Returns `false` for all `Authentication` variants that are not `Authentication::Keyring`
    ///
    /// # Errors
    /// Returns an error if there was an error accessing the keyring (besides not existing)
    pub fn keyring_entry_exists(&self) -> Result<bool, InstrumentError> {
        match self {
            Self::Keyring { id } => {
                let entry = keyring::Entry::new(SERVICE_NAME, id)?;
                match entry.get_secret() {
                    Ok(_) => Ok(true),
                    Err(keyring::Error::NoEntry) => Ok(false),
                    Err(e) => Err(e.into()),
                }
            }
            _ => Ok(false),
        }
    }

    /// Saves this credential to the system keyring. This will overwrite an existing
    /// entry or create a new one if it doesn't already exist.
    ///
    /// # Errors
    /// Errors may occur from the interactions with the [`keyring`] crate.
    pub fn save_credential(&self, model: &Model, serial: &str) -> Result<(), InstrumentError> {
        #[cfg(test)] // Don't use the system credential manager for unit tests.
        set_default_credential_builder(mock::default_credential_builder());

        let name = format!("{model}#{serial}");
        let (username, password) = match self {
            Self::Prompt => {
                return Err(InstrumentError::AuthenticationFailure(
                    "no credentials provided".to_string(),
                ))
            }
            Self::PromptPartial { username, password } => (
                username.clone().unwrap_or_default(),
                password.clone().unwrap_or_default(),
            ),
            Self::Credential { username, password } => (username.to_string(), password.to_string()),
            Self::Keyring { id } => {
                let entry = keyring::Entry::new(SERVICE_NAME, id)?;
                let secret = &entry.get_secret()?;
                let secret: SecretEntry =
                    serde_json::from_str(String::from_utf8_lossy(secret).as_ref())?;
                (secret.username, secret.password)
            }
            Self::NoAuth => return Ok(()),
        };
        let secret = SecretEntry { username, password };

        let entry = keyring::Entry::new(SERVICE_NAME, &name)?;

        entry.set_secret(serde_json::to_string(&secret)?.as_bytes())?;

        Ok(())
    }
}
