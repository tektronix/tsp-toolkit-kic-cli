use crate::debugger::breakpoint::Breakpoint;
use crate::debugger::variable::VariableInfo;
use crate::debugger::watchpoint::WatchpointInfo;

// use crate::TspError;
//
/// A request from a user that is to be dispatched within the program.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Request {
    /// A TSP command that should be sent to the instrument
    Tsp(String),
    /// A request for the errors from the debugger.
    GetError(String),
    BreakPoint {
        breakpoint_info: Breakpoint,
    },
    StartDebugger {
        file_path: String,
        break_points: Vec<Breakpoint>,
    },
    Watchpoint {
        watchpoint_info: WatchpointInfo,
    },
    Variable {
        vairable_info: VariableInfo,
    },
    Run,
    StepOver,
    StepIn,
    StepOut,
    ClearBreakPoints,
    Exit,
    Help {
        sub_cmd: Option<String>,
    },
    Usage(String),
    None,
}
