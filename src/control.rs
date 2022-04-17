use crate::mi;
use crate::Arc;
use crate::ConsoleOutput;
use crate::ControlState::{AttachFileDialog, SendCommand};
use snailquote::unescape;
use std::cell::RefCell;
use std::rc::Rc;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ControlState {
    LookingForGDB {
        sent_command: bool,
    },
    GDBNotFound,
    StartGDB {
        sent_command: bool,
    },
    GDBNothingLoaded,
    AttachFileDialog {
        sent_command: u32,
        path: Option<String>,
    },
    TryAttachPort {
        sent_command: u32,
        host: Option<String>,
    },

    GDBRunning {
        state: GDBExecutionState,
        last_output: Option<mi::Output>,
    },

    SendCommand {
        commands: Vec<String>,
        check: Arc<dyn FnOnce(ControlState, ConsoleOutput) -> ControlState>, //commands: Vec<&'static str>,
        sent: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GDBExecutionState {
    Running,
    Stopped,
    Unknown,
}

impl ControlState {
    pub fn new() -> ControlState {
        ControlState::LookingForGDB {
            sent_command: false,
        }
    }

    pub fn buttons(&self) -> Vec<String> {
        use ControlState::*;
        match self {
            GDBNothingLoaded => ["Attach to port (QEMU)", "Load binary"].to_cmds(),
            AttachFileDialog {
                sent_command: 0, ..
            } => ["Load"].to_cmds(),

            TryAttachPort {
                sent_command: 0, ..
            } => ["Connect"].to_cmds(),
            _ => vec![],
        }
    }

    pub fn buttons_n(&self) -> &[(&str, fn(&ControlState) -> ControlState)] {
        use ControlState::*;
        match self {
            GDBNothingLoaded => &[(
                "Attach to port (QEMU)",
                |old: &ControlState| -> ControlState { ControlState::GDBNotFound },
            )],
            _ => &[],
        }
    }

    pub fn input_fields(&self) -> Vec<(String, String)> {
        use ControlState::*;
        match self {
            AttachFileDialog {
                sent_command: 0 | 1,
                ..
            } => vec![("Filename".into(), "type binary path".into())],

            TryAttachPort {
                sent_command: 0 | 1,
                ..
            } => vec![("Host Address".into(), "type the host's addr".into())],
            _ => vec![],
        }
    }

    fn no_stderr(next: ControlState) -> impl FnOnce(ControlState, ConsoleOutput) -> ControlState {
        move |state, input| match input {
            ConsoleOutput::Stdout(_) => next.clone(),
            ConsoleOutput::Stderr(e) => panic!("{}", e),
        }
    }

    fn send_commands(
        cmds: &[&str],
        check: Arc<dyn FnOnce(ControlState, ConsoleOutput) -> ControlState>,
    ) -> ControlState {
        SendCommand {
            commands: cmds.into_iter().map(|&s| s.into()).collect(),
            sent: false,
            check: check.clone(),
        }
    }
}

pub fn user_output(src: &str) -> Option<String> {
    let mut src = unescape(src).unwrap();

    let src = if src.ends_with('\n') {
        format!("{}\n", src.trim_end())
    } else {
        src
    };

    match mi::output_kind(&src) {
        Some((mi::Output::ConsoleStream, _)) => Some(src[1..].to_string()),
        //None => Some(src),
        _ => None,
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
                Arc::new(ControlState::no_stderr(StartGDB {
                    sent_command: false,
                })),
            ),
            vec![],
        ),
        //        LookingForGDB {
        //            sent_command: false,
        //        } => (
        //            LookingForGDB { sent_command: true },
        //            ["gdb --version", "which gdb"].to_cmds(),
        //        ),
        //
        //        StartGDB {
        //            sent_command: false,
        //        } => (
        //            StartGDB { sent_command: true },
        //            ["gdb --interpreter=mi"].to_cmds(),
        //        ),
        //
        //        AttachFileDialog {
        //            sent_command: 1,
        //            path: Some(p),
        //        } => (
        //            AttachFileDialog {
        //                sent_command: 2,
        //                path: Some(p.clone()),
        //            },
        //            vec![format!("file {p}"), "start".into()],
        //        ),
        //
        //        TryAttachPort {
        //            sent_command: 1,
        //            host: Some(h),
        //        } => (
        //            TryAttachPort {
        //                sent_command: 2,
        //                host: Some(h.clone()),
        //            },
        //            vec![format!("target remote {h}")],
        //        ),
        //
        SendCommand {
            commands: cmds,
            check: f,
            sent: false,
        } => (
            SendCommand {
                commands: cmds.clone(),
                check: *f,
                sent: true,
            },
            cmds.clone(),
        ),
        _ => (state.clone(), vec![]),
    }
}

pub fn read_console_input(state: ControlState, input: &ConsoleOutput) -> ControlState {
    use ConsoleOutput::*;
    use ControlState::*;

    match state {
        //LookingForGDB { sent_command: true } => match input {
        //    Stdout(_) => StartGDB {
        //        sent_command: false,
        //    },
        //    Stderr(_) => GDBNotFound,
        //},

        //StartGDB { sent_command: true } => match input {
        //    Stdout(_) => GDBNothingLoaded,
        //    Stderr(e) => panic!("Can't start GDB {e}"),
        //},

        //AttachFileDialog {
        //    sent_command: 2, ..
        //} => GDBRunning {
        //    state: GDBExecutionState::Unknown,
        //    last_output: None,
        //},
        SendCommand {
            check: f,
            sent: true,
            ..
        } => f(state, *input),

        GDBRunning {
            state: s,
            last_output: last,
        } => match input {
            Stdout(input) => {
                let output = mi::parse(input);
                GDBRunning {
                    state: s,
                    last_output: if !output.is_none() { output } else { last },
                }
            }
            Stderr(e) => panic!("{}", e),
        },

        _ => state,
    }
}

pub fn read_button_input(
    state: ControlState,
    buttons: &[bool],
    input_fields: &[String],
) -> ControlState {
    let opts = state.buttons_n();

    let mut next = state.clone();
    for (btn, (_, f)) in buttons.iter().zip(opts) {
        if *btn {
            next = f(&state);
        }
    }

    next

    //use ControlState::*;
    //match state {
    //    GDBNothingLoaded => {
    //        if buttons[0] {
    //            TryAttachPort {
    //                sent_command: 0,
    //                host: None,
    //            }
    //        } else if buttons[1] {
    //            AttachFileDialog {
    //                sent_command: 0,
    //                path: None,
    //            }
    //        } else {
    //            GDBNothingLoaded
    //        }
    //    }

    //    AttachFileDialog {
    //        sent_command: 0,
    //        ref path,
    //    } => {
    //        if buttons[0] {
    //            AttachFileDialog {
    //                sent_command: 1,
    //                path: Some(input_fields[0].clone()),
    //            }
    //        } else {
    //            state.clone()
    //        }
    //    }

    //    TryAttachPort {
    //        sent_command: 0,
    //        ref host,
    //    } => {
    //        if buttons[0] {
    //            TryAttachPort {
    //                sent_command: 1,
    //                host: Some(input_fields[0].clone()),
    //            }
    //        } else {
    //            state.clone()
    //        }
    //    }

    //    _ => state,
    //}
}
