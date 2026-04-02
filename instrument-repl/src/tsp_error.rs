use std::fmt::Display;

use serde::de;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct InstrumentTime {
    secs: u64,
    #[serde(deserialize_with = "deserialize_fractional")]
    nanos: f64,
}

/// Make sure we properly handle fractional seconds on TTI being and integer number of
/// nanos.
fn deserialize_fractional<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: de::Deserializer<'de>,
{
    let value: f64 = de::Deserialize::deserialize(deserializer)?;
    if value > 1.0 {
        // This is an integer number of nanos from TTI
        Ok(value * 1e-9)
    } else {
        // This is fractional seconds from MP5000
        Ok(value)
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TspError {
    error_code: i64,
    message: String,
    severity: u8,
    node_id: i16,
    time: Option<InstrumentTime>,
}

impl Display for TspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.error_code;
        let msg = &self.message;
        //let _sev = self.severity;
        let node = self.node_id;
        // let _time: String = match self.time {
        //     Some(InstrumentTime::Tti { secs, nanos }) => format!("{secs}.{nanos:09}"),
        //     None => String::new(),
        // };
        write!(f, "[{node}] {{{id}}} {msg}")
    }
}

#[cfg(test)]
mod unit {
    use crate::{InstrumentTime, TspError};

    #[test]
    fn parse_tti_instrument_time() {
        let input = r#"{"secs": 1773416437,"nanos": 4570421}"#;
        let expected = InstrumentTime {
            secs: 1773416437,
            nanos: { 4570421.0 * 1e-9 },
        };
        let actual: InstrumentTime =
            serde_json::from_str(&input).expect("should parse instrument time");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_mp5000_instrument_time() {
        let input = r#"{"secs": 1773416437,"nanos": 0.4570421}"#;
        let expected = InstrumentTime {
            secs: 1773416437,
            nanos: { 0.4570421 },
        };
        let actual: InstrumentTime =
            serde_json::from_str(&input).expect("should parse instrument time");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_tti_tsp_error() {
        let input = r#"{"message": "TSP Syntax error at line 2: `=' expected near `<eof>'","node_id": 1,"time": {"secs": 1773416437,"nanos": 4570421},"severity": 20,"error_code": -285}"#;
        let expected = TspError {
            error_code: -285,
            message: "TSP Syntax error at line 2: `=' expected near `<eof>'".to_string(),
            severity: 20,
            node_id: 1,
            time: Some(InstrumentTime {
                secs: 1773416437,
                nanos: { 4570421.0 * 1e-9 },
            }),
        };
        let actual: TspError = serde_json::from_str(&input).expect("should parse TspError");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_mp5000_tsp_error() {
        let input = r#"{"message": "TSP Syntax error at line 2: `=' expected near `<eof>'","node_id": 1,"time": {"secs": 1773416437,"nanos": 0.4570421},"severity": 20,"error_code": -285}"#;
        let expected = TspError {
            error_code: -285,
            message: "TSP Syntax error at line 2: `=' expected near `<eof>'".to_string(),
            severity: 20,
            node_id: 1,
            time: Some(InstrumentTime {
                secs: 1773416437,
                nanos: 0.4570421,
            }),
        };
        let actual: TspError = serde_json::from_str(&input).expect("should parse TspError");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_2600_tsp_error() {
        let input = r#"{"message": "TSP Syntax error at line 2: `=' expected near `<eof>'","node_id": 1, "severity": 20,"error_code": -285}"#;
        let expected = TspError {
            error_code: -285,
            message: "TSP Syntax error at line 2: `=' expected near `<eof>'".to_string(),
            severity: 20,
            node_id: 1,
            time: None,
        };
        let actual: TspError = serde_json::from_str(&input).expect("should parse TspError");
        assert_eq!(actual, expected);
    }
}
