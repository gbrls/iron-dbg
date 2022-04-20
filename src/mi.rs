use anyhow::anyhow;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::char;
use nom::combinator::{map, map_res, opt};
use nom::sequence::{delimited, preceded};
use nom::{Err, IResult, Parser};
use snailquote::unescape;

#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    ///  # **Symbol**: `+`  
    ///  On-going status information about the progress of a slow operation. **It can be discarded.**
    StatusAsync,
    /// # **Symbol**: `*`  
    /// asynchronous state change on the target (stopped, started, disappeared).  
    /// Docs: https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Async-Records.html
    ExecAsync {
        state: Option<ExecutionState>,
        rest: String,
    },
    /// # **Symbol**: `=`  
    /// supplementary information that the client should handle (e.g., a new breakpoint information).
    NotifyAsync,
    /// # **Symbol**: `~`  
    /// should be displayed as is in the console. It is the textual response to a CLI command.
    ConsoleStream(String),
    /// # **Symbol**: `@`  
    /// output produced by the target program
    TargetStream(String),
    /// # **Symbol**: `&`  
    /// text coming from GDB’s internals, for instance messages that should be displayed as part of an error log.
    LogStream(String),
    /// # **Symbol**: `^`  
    /// No doc from GDB MI
    ResultRecord(MIResult),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MIResult {
    Done,
    Running,
    Connected,
    Error { msg: String, code: Option<String> },
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncStateStatus {
    Running {
        thread: String,
    },
    Stopped {
        reason: StoppedReason,
        thread: String,
        stopped_threads: String,
        core: String,
        frame: Option<Frame>,
    },
}

/// [docs](https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Async-Records.html#GDB_002fMI-Async-Records)
#[derive(Debug, Clone, PartialEq)]
pub enum AsyncInfo {
    ThreadGroupAdded {
        id: String,
    },
    ThreadGroupRemoved {
        id: String,
    },
    ThreadGroupStarted {
        id: String,
        pid: String,
    },
    ThreadGroupExited {
        id: String,
        exit_code: Option<String>,
    },
    ThreadCreated {
        id: String,
        group_id: String,
    },
    ThreadExited {
        id: String,
        group_id: String,
    },
    ThreadSelected {
        id: String,
        frame: Option<Frame>,
    },
    LibraryLoaded {
        id: String,
        target_name: String,
        host_name: String,
        symbols_loaded: String,
        ranges: String,
        thread_group: Option<String>,
    },
    LibraryUnloaded {
        id: String,
        target_name: String,
        host_name: String,
        thread_group: Option<String>,
    },
    /// Either:
    /// =traceframe-changed,num=tfnum,tracepoint=tpnum
    /// =traceframe-changed,end
    TraceframeChanged {
        num: Option<String>,
        tracepoint: Option<String>,
    },
    TSVCreated {
        name: String,
        initial: String,
    },
    TSVDeleted {
        name: Option<String>,
    },
    TSVModified {
        name: String,
        initial: String,
        current: Option<String>,
    },
    BreakpointCreated {
        bkpt: Breakpoint,
    },
    BreakpointModified {
        bkpt: Breakpoint,
    },
    BreakpointDeleted {
        id: String,
    },
    RecordStarted {
        thread_group: String,
        method: String,
        format: Option<String>,
    },
    RecordStopped {
        thread_group: String,
    },
    CmdParamChanged {
        param: String,
        value: String,
    },
    MemoryChanged {
        thread_group: String,
        addr: u64,
        len: u32,
        m_type: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StoppedReason {
    BreakpointHit,
    WatchpointTrigger,
    AccessWatchpointTrigger,
    FunctionFinished,
    LocationReached,
    WatchPointScope,
    EndSteppingRange,
    ExitSignalled,
    Exited,
    ExitedNormally,
    SignalReceived,
    SolibEvent,
    Fork,
    VFork,
    SyscallEntry,
    SyscallReturn,
    Exec,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    Done,
    Running,
    Connected,
    Error { msg: String },
    Exit,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    addr: u64,
    func: String,
    args: Vec<String>,
    file: String,
    fullname: String,
    line: u32,
    arch: String,
    level: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Breakpoint {
    number: String,
    b_type: String,
    disp: String,
    enabled: bool,
    addr: u64,
    func: String,
    file: String,
    fullname: String,
    line: u32,
    thread_groups: Vec<String>,
    times: String,
}

pub fn parse(input: &str) -> IResult<&str, Output> {
    use nom::combinator::map;
    let (rest, out) = alt((
        map(tag("="), |_| Output::NotifyAsync),
        // Stream records
        map(preceded(tag("~"), owned_unescaped), Output::ConsoleStream),
        map(preceded(tag("@"), owned_unescaped), Output::TargetStream),
        map(preceded(tag("&"), owned_unescaped), Output::LogStream),
        // ResultRecord
        map(preceded(tag("^"), mi_result), Output::ResultRecord),
    ))(input)?;

    Ok((rest, out))
}

/// [docs](https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Result-Records.html#GDB_002fMI-Result-Records)
fn mi_result(input: &str) -> IResult<&str, MIResult> {
    let (rest, miresult) = alt((
        tag("done"),
        tag("running"),
        tag("connected"),
        tag("error"),
        tag("exit"),
    ))(input)?;

    match miresult {
        "done" => Ok((rest, MIResult::Done)),
        "running" => Ok((rest, MIResult::Running)),
        "connected" => Ok((rest, MIResult::Connected)),
        "error" => {
            let (rest, _) = tag(",")(rest)?;
            let (_, (msg, code)) = message(rest)?;
            Ok((
                rest,
                MIResult::Error {
                    msg: msg.to_string(),
                    code: code.and_then(|s| Some(s.to_string())),
                },
            ))
        }
        "exit" => Ok((rest, MIResult::Exit)),
        _ => todo!("Handle error better"),
    }
}

fn message(input: &str) -> IResult<&str, (&str, Option<&str>)> {
    let (rest, _) = tag("msg=")(input)?;
    let (rest, msg) = c_str(rest)?;
    let (rest, code) = opt(preceded(tag(",code="), c_str))(rest)?;

    Ok((rest, (msg, code)))
}

fn owned_unescaped(input: &str) -> IResult<&str, String> {
    //let (rest, s) = delimited(char('\"'), take_while(|c| c != '\"'), char('\"'))(input)?;

    //Ok((rest, unescape(s).unwrap()))
    //TODO: this doesn't return the correct rest
    Ok((input, unescape(input).unwrap()))
}

fn c_str(input: &str) -> IResult<&str, &str> {
    let (rest, s) = delimited(char('\"'), take_while(|c| c != '\"'), char('\"'))(input)?;

    //Ok((rest, &unescape(s).unwrap()))
    Ok((rest, s))
}

pub fn user_output(src: &str) -> Option<String> {
    let p = parse(src);

    if let Ok((_, Output::ConsoleStream(src))) = p {
        let src = if src.ends_with('\n') {
            format!("{}\n", src.trim_end())
        } else {
            src
        };
        Some(src)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_result() {
        assert_eq!(mi_result("done").unwrap().1, MIResult::Done,);
        assert_eq!(
            mi_result("error,msg=\"Alguma mensagem de erro\"")
                .unwrap()
                .1,
            MIResult::Error {
                msg: "Alguma mensagem de erro".to_string(),
                code: None
            },
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            parse("^done").unwrap().1,
            Output::ResultRecord(MIResult::Done)
        );

        assert_eq!(
            parse("^error,msg=\"Alguma mensagem de erro\"").unwrap().1,
            Output::ResultRecord(MIResult::Error {
                msg: "Alguma mensagem de erro".to_string(),
                code: None
            }),
        );
    }
}
