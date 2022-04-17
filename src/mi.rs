use crate::mi;

pub enum MIVal {
    Done,
    Err,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    ///  **Symbol**: `+`  
    ///  On-going status information about the progress of a slow operation. **It can be discarded.**
    StatusAsync,
    /// **Symbol**: `*`
    /// asynchronous state change on the target (stopped, started, disappeared).  
    /// Docs: https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Async-Records.html
    ExecAsync {
        state: Option<ExecutionState>,
        rest: String,
    },
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
    Response { result: Option<ExecutionState> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    Done,
    Running,
    Connected,
    Error,
    Exit,
    Stopped,
}

fn execution_state(input: &str) -> Option<(ExecutionState, usize)> {
    let pats = [
        (ExecutionState::Done, "done"),
        (ExecutionState::Running, "running"),
        (ExecutionState::Connected, "connected"),
        (ExecutionState::Error, "error"),
        (ExecutionState::Exit, "exit"),
        (ExecutionState::Stopped, "stopped"),
    ];

    match pats.into_iter().find(|x| input.starts_with(x.1)) {
        Some(el) => Some((el.0, el.1.len())),
        None => None,
    }
}

pub fn output_kind(input: &str) -> Option<(Output, usize)> {
    use Output::*;

    if input.is_empty() {
        return None;
    }

    match input.chars().next().unwrap() {
        '+' => Some((StatusAsync, 1)),
        '*' => Some((
            ExecAsync {
                state: None,
                rest: String::new(),
            },
            1,
        )),
        '=' => Some((NotifyAsync, 1)),
        '~' => Some((ConsoleStream, 1)),
        '@' => Some((TargetStream, 1)),
        '&' => Some((LogStream, 1)),
        '^' => Some((Response { result: None }, 1)),

        _ => None,
    }
}

pub fn apply_parser<T>(f: fn(&str) -> Option<(T, usize)>, input: &str) -> Option<(T, &str)> {
    if let Some((val, read)) = f(input) {
        Some((val, &input[read..]))
    } else {
        None
    }
}

pub fn parse(input: &str) -> Option<Output> {
    use Output::*;

    match apply_parser(output_kind, input) {
        Some((Response { result: None }, input)) => {
            let res = if let Some((response_result, input)) = apply_parser(execution_state, input) {
                Some(response_result)
            } else {
                None
            };
            Some(Response { result: res })
        }

        Some((
            ExecAsync {
                state: None,
                rest: r,
            },
            input,
        )) => {
            let (state, rest_input) =
                if let Some((state, rest_input)) = apply_parser(execution_state, input) {
                    (Some(state), rest_input)
                } else {
                    (None, "")
                };

            Some(
                (ExecAsync {
                    state,
                    rest: rest_input.to_string(),
                }),
            )
        }

        Some((x, _)) => Some(x),
        None => None,
    }
    //if let Some((Output::Response, input)) = apply_parser(output_kind, input) {
    //    let (resp, input) = apply_parser(response_result, input).unwrap();
    //    println!("[Resp] {resp:?}");
    //}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_result() {
        assert_eq!(
            apply_parser(execution_state, "done").unwrap().0,
            ExecutionState::Done
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            parse("^done"),
            Some(Output::Response {
                result: Some(ExecutionState::Done)
            })
        );

        assert_eq!(
            parse("^running"),
            Some(Output::Response {
                result: Some(ExecutionState::Running)
            })
        );
    }
}
