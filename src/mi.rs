pub enum MIVal {
    Done,
    Err,
}

pub enum OutputKind {
    /// **Symbol**: `+`  
    /// On-going status information about the progress of a slow operation. **It can be discarded.**
    StatusAsync,
    /// **Symbol**: `*`
    /// asynchronous state change on the target (stopped, started, disappeared).
    ExecAsync,
    /// **Symbol**: `=`
    /// supplementary information that the client should handle (e.g., a new breakpoint information).
    NotifyAsync,
    /// **Symbol**: `~`
    /// should be displayed as is in the console. It is the textual response to a CLI command.
    ConsoleStream,
    /// **Symbol**: `@`
    /// output produced by the target program
    TargetStream,
    /// **Symbol**: `&`
    /// text coming from GDB’s internals, for instance messages that should be displayed as part of an error log.
    LogStream,
    /// **Symbol**: `^`
    /// No doc from GDB MI
    Response,
}

pub enum ResponseResult {
    Done,
    Running,
    Connected,
    Error,
    Exit,
}

fn response_result(input: &str) -> Option<(ResponseResult, usize)> {
    let pats = [
        (ResponseResult::Done, "done"),
        (ResponseResult::Running, "running"),
        (ResponseResult::Connected, "connected"),
        (ResponseResult::Error, "error"),
        (ResponseResult::Exit, "exit"),
    ];

    match pats.into_iter().find(|x| input.starts_with(x.1)) {
        Some(el) => Some((el.0, el.1.len())),
        None => None,
    }
}

pub fn output_kind(input: &str) -> Option<(OutputKind, usize)> {
    use OutputKind::*;

    if input.is_empty() {
        return None;
    }

    match input.chars().next().unwrap() {
        '+' => Some((StatusAsync, 1)),
        '*' => Some((ExecAsync, 1)),
        '=' => Some((NotifyAsync, 1)),
        '~' => Some((ConsoleStream, 1)),
        '@' => Some((TargetStream, 1)),
        '&' => Some((LogStream, 1)),
        '^' => Some((Response, 1)),

        _ => None,
    }
}

pub fn parse(input: &str) -> MIVal {
    use MIVal::*;

    Done
}
