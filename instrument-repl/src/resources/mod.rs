use std::fmt::Display;

use crate::VERSION;
const VERSION_REPLACE: &str = "!<!<VERSION>!>!";

pub const KIC_COMMON_TSP: Resource = Resource {
    source: include_str!("./kic_common.tsp"),
};

pub const TSP_LINK_NODES_TSP: Resource = Resource {
    source: include_str!("./TspLinkNodeDetails.tsp"),
};

/// A resource that can be used as-is
#[derive(Debug)]
pub struct Resource {
    /// The raw resource that can be used as-is
    source: &'static str,
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = self.source.replace(VERSION_REPLACE, VERSION);
        write!(f, "{source}")
    }
}

#[cfg(test)]
mod unit {
    use crate::{resources::Resource, VERSION};

    #[test]
    fn replace_version() {
        const TEST_FILE: Resource = Resource {
            source: "_KIC = {\n    version = \"!<!<VERSION>!>!\"\n}\n",
        };

        let expected: String = format!("_KIC = {{\n    version = \"{VERSION}\"\n}}\n");

        assert_eq!(TEST_FILE.to_string(), expected);
    }
}
