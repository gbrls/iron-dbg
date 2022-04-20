use crate::mi;
use crate::Arc;
use crate::ConsoleOutput;
use crate::ControlState::{AttachFileDialog, SendCommand};
use snailquote::unescape;
use std::cell::RefCell;
use std::fmt;
use std::fmt::{write, Formatter};
use std::rc::Rc;

use crate::mi::{parse, ExecutionState, Output};
use static_init::dynamic;

/// We need to keep track of the commands we sent to the shell to be able to backtrack errors and
/// restart a shell and go back to a known state
#[dynamic(drop)]
static mut CMD_HISTORY: Vec<String> = Vec::new();

trait ToCommandVec {
    fn to_cmds(self) -> Vec<String>;
}

impl ToCommandVec for &[&str] {
    fn to_cmds(self) -> Vec<String> {
        self.into_iter().map(|s| String::from(*s)).collect()
    }
}

#[derive(Debug, Clone)]
pub enum InputCommand {
    StdinInput(String),
}

#[derive(Clone)]
struct BoxedFn(Arc<dyn Fn(ControlState, ConsoleOutput) -> ControlState + Send + Sync>);

impl BoxedFn {
    fn call(&self, a: ControlState, b: ConsoleOutput) -> ControlState {
        if let BoxedFn(f) = self {
            (f)(a, b)
        } else {
            panic!("WTF")
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ControlState {
    LookingForGDB,
    GDBNotFound,
    StartGDB,
    GDBNothingLoaded,
    AttachFileDialog {
        path: Option<String>,
    },
    TryAttachPort {
        host: Option<String>,
    },

    GDBRunning {
        state: GDBExecutionState,
        last_output: Option<mi::Output>,
    },

    SendCommand {
        commands: Vec<String>,
        check: BoxedFn, //commands: Vec<&'static str>,
        sent: bool,
    },

    RestartAndRecover {
        sent: bool,
        prev: Box<ControlState>,
    },
}

impl fmt::Debug for BoxedFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#function")
    }
}

impl PartialEq for BoxedFn {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GDBExecutionState {
    Running,
    Stopped,
    Unknown,
}

fn execution_state_from_output(cur: &GDBExecutionState, output: &mi::Output) -> GDBExecutionState {
    match output {
        mi::Output::ExecAsync { state: Some(s), .. } => match s {
            ExecutionState::Running | ExecutionState::Connected => GDBExecutionState::Running,
            ExecutionState::Stopped => GDBExecutionState::Stopped,
            _ => GDBExecutionState::Unknown,
        },
        _ => *cur,
    }
}

impl ControlState {
    pub fn new() -> ControlState {
        ControlState::LookingForGDB
    }

    pub fn buttons(&self) -> &[(&str, fn(&ControlState, &[String]) -> ControlState)] {
        use ControlState::*;
        match self {
            GDBNothingLoaded => &[
                ("Attach to port (QEMU)", |_, _| -> ControlState {
                    ControlState::TryAttachPort { host: None }
                }),
                ("Load binary", |_, str_in| -> ControlState {
                    AttachFileDialog { path: None }
                }),
            ],
            AttachFileDialog { path: None } => &[("Load", |_, str_in| AttachFileDialog {
                path: Some(str_in[0].clone()),
            })],

            TryAttachPort { host: None } => &[("Connect", |_, str_in| TryAttachPort {
                host: Some(str_in[0].clone()),
            })],

            GDBRunning { .. } => &[("Reload", |prev, _| RestartAndRecover {
                sent: false,
                prev: Box::new(prev.clone()),
            })],

            _ => &[],
        }
    }

    pub fn input_fields(&self) -> &[(&str, &str)] {
        use ControlState::*;
        match self {
            AttachFileDialog { path: None } => &[("Filename", "./res/a.out")],

            TryAttachPort { host: None } => &[("Host Address", "127.0.0.1:1234")],
            _ => &[],
        }
    }

    // @TODO: handle GDB errors here from the STDOUT
    fn no_stderr(next: ControlState) -> impl Fn(ControlState, ConsoleOutput) -> ControlState {
        move |state, input| match input {
            ConsoleOutput::Stdout(_) => next.clone(),
            ConsoleOutput::Stderr(e) => panic!("{}", e),
        }
    }

    fn send_commands(
        cmds: &[&str],
        check: impl Fn(ControlState, ConsoleOutput) -> ControlState + Sync + Send + 'static,
    ) -> ControlState {
        SendCommand {
            commands: cmds.into_iter().map(|&s| s.into()).collect(),
            sent: false,
            check: BoxedFn(Arc::new(check)),
        }
    }
}

// (Kernel debugging) Attach to a running QEMU instance.
// (Userspace debugging) Run an executable file.
fn try_run() -> Option<()> {
    None
}

pub fn advance_cmds(state: &ControlState) -> (ControlState, Vec<String>) {
    use ControlState::*;

    match state {
        LookingForGDB { .. } => (
            ControlState::send_commands(
                &["gdb --version", "which gdb"],
                ControlState::no_stderr(StartGDB),
            ),
            vec![],
        ),
        StartGDB { .. } => (
            ControlState::send_commands(
                &["gdb --interpreter=mi3"],
                ControlState::no_stderr(GDBNothingLoaded),
            ),
            vec![],
        ),

        AttachFileDialog { path: Some(p) } => (
            ControlState::send_commands(
                &[&format!("file {p}"), "start"],
                ControlState::no_stderr(GDBRunning {
                    state: GDBExecutionState::Unknown,
                    last_output: None,
                }),
            ),
            vec![],
        ),

        TryAttachPort { host: Some(h) } => (
            ControlState::send_commands(
                &[&format!("target remote {h}")],
                ControlState::no_stderr(GDBRunning {
                    state: GDBExecutionState::Unknown,
                    last_output: None,
                }),
            ),
            vec![],
        ),
        SendCommand {
            commands: cmds,
            check: f,
            sent: false,
        } => {
            unsafe {
                let mut lock = CMD_HISTORY.write();
                for cmd in cmds {
                    lock.push(cmd.clone());
                }
            }
            (
                SendCommand {
                    commands: cmds.clone(),
                    check: f.clone(),
                    sent: true,
                },
                cmds.clone(),
            )
        }

        RestartAndRecover { sent: false, prev } => unsafe {
            let mut cmds = vec!["quit".to_string(), "pwd".to_string()];
            let len = CMD_HISTORY.read().len();
            for cmd in &CMD_HISTORY.read()[0..len] {
                cmds.push(cmd.clone());
            }

            (*(*prev).clone(), cmds)
        },
        _ => (state.clone(), vec![]),
    }
}

pub fn read_console_input(state: ControlState, input: &ConsoleOutput) -> ControlState {
    use ConsoleOutput::*;
    use ControlState::*;

    match state {
        SendCommand {
            check: BoxedFn(ref verify),
            sent: true,
            ..
        } => verify(state.clone(), input.clone()),

        GDBRunning {
            state: s,
            last_output: last,
        } => match input {
            Stdout(input) => {
                let output = mi::parse(input);
                GDBRunning {
                    state: if output.is_ok() {
                        execution_state_from_output(&s, &output.as_ref().unwrap().1)
                    } else {
                        s
                    },
                    last_output: if output.is_ok() {
                        Some(output.unwrap().1)
                    } else {
                        last
                    },
                }
            }
            Stderr(e) => panic!("{}", e),
        },

        _ => state,
    }
}
