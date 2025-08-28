use std::fmt::Display;

use crate::{error::Result, VERSION};
const VERSION_REPLACE: &str = "!<!<VERSION>!>!";

pub const KIDEBUGGER_TSP: EncryptedResource = EncryptedResource {
    source: include_bytes!("./kiDebugger.tsp"),
};

pub const TSPDBG_TSP: EncryptedResource = EncryptedResource {
    source: include_bytes!("./tspdbg.tsp"),
};

/// An encrypted resource that needs to be decrypted in order to work.
#[derive(Debug)]
pub struct EncryptedResource {
    /// The raw, encrypted resource.
    source: &'static [u8],
}

impl EncryptedResource {
    /// Decrypt the given encrypted resource
    ///
    /// # Errors
    /// An error may occur if the encrypted resource could not be decrypted successfully.
    pub fn decrypt(self) -> Result<Resource> {
        Ok(Resource {
            //TODO
            source: self.source.to_vec(),
        })
    }
}
/// A resource that can be used as-is
#[derive(Debug)]
pub struct Resource {
    /// The raw resource that can be used as-is
    source: Vec<u8>,
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = String::from_utf8_lossy(&self.source);
        let source = source.replace(VERSION_REPLACE, VERSION);
        write!(f, "{source}")
    }
}

#[cfg(test)]
mod unit {
    use crate::{resources::EncryptedResource, VERSION};

    #[test]
    fn decrypt() {
        const TEST_FILE: EncryptedResource = EncryptedResource {
            source: b"Hello World!",
        };
        let expected: String = "Hello World!".to_string();

        assert_eq!(TEST_FILE.decrypt().unwrap().to_string(), expected);
    }

    #[test]
    fn replace_version() {
        const TEST_FILE: EncryptedResource = EncryptedResource {
            source: b"_KIC = {\n    version = \"!<!<VERSION>!>!\"\n}\n",
        };

        let expected: String = format!("_KIC = {{\n    version = \"{VERSION}\"\n}}\n");

        assert_eq!(TEST_FILE.decrypt().unwrap().to_string(), expected);
    }
}
