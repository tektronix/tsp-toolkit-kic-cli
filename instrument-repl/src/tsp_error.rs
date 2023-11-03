use std::fmt::Display;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct InstrumentTime {
    secs: u64,
    nanos: u64,
}

#[derive(serde::Deserialize, serde::Serialize)]
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
