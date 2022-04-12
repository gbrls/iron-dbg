use crate::mi;
use crate::ConsoleOutput;
use crate::ControlState::AttachFileDialog;
use snailquote::unescape;

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
    GDBStarted,
    AttachFileDialog {
        sent_command: u32,
        path: Option<String>,
    },
    TryAttachPort {
        sent_command: u32,
        host: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum GDBExecutionState {}

impl ControlState {
    pub fn new() -> ControlState {
        ControlState::LookingForGDB {
            sent_command: false,
        }
    }

    pub fn buttons(&self) -> Vec<String> {
        use ControlState::*;
        match self {
            GDBStarted => ["Attach to port (QEMU)", "Load binary"].to_cmds(),
            AttachFileDialog {
                sent_command: 0, ..
            } => ["Load"].to_cmds(),

            TryAttachPort {
                sent_command: 0, ..
            } => ["Connect"].to_cmds(),
            _ => vec![],
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
}

pub fn user_output(src: &str) -> Option<String> {
    let mut src = unescape(src).unwrap();

    let src = if src.ends_with('\n') {
        format!("{}\n", src.trim_end())
    } else {
        src
    };

    match mi::output_kind(&src) {
        Some((mi::OutputKind::ConsoleStream, _)) => Some(src[1..].to_string()),
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
        LookingForGDB {
            sent_command: false,
        } => (
            LookingForGDB { sent_command: true },
            ["gdb --version", "which gdb"].to_cmds(),
        ),

        StartGDB {
            sent_command: false,
        } => (
            StartGDB { sent_command: true },
            ["gdb --interpreter=mi"].to_cmds(),
        ),

        AttachFileDialog {
            sent_command: 1,
            path: Some(p),
        } => (
            AttachFileDialog {
                sent_command: 2,
                path: Some(p.clone()),
            },
            vec![format!("file {p}"), "start".into()],
        ),

        TryAttachPort {
            sent_command: 1,
            host: Some(h),
        } => (
            TryAttachPort {
                sent_command: 2,
                host: Some(h.clone()),
            },
            vec![format!("target remote {h}")],
        ),

        _ => (state.clone(), vec![]),
    }
}

pub fn read_console_input(state: ControlState, input: &ConsoleOutput) -> ControlState {
    use ConsoleOutput::*;
    use ControlState::*;

    match state {
        LookingForGDB { sent_command: true } => match input {
            Stdout(_) => StartGDB {
                sent_command: false,
            },
            Stderr(_) => GDBNotFound,
        },

        StartGDB { sent_command: true } => match input {
            Stdout(_) => GDBStarted,
            Stderr(e) => panic!("Can't start GDB {e}"),
        },

        AttachFileDialog {
            sent_command: 2, ..
        } => AttachFileDialog {
            sent_command: 0,
            path: None,
        },

        _ => state,
    }
}

pub fn read_button_input(
    state: ControlState,
    buttons: &[bool],
    input_fields: &[String],
) -> ControlState {
    use ControlState::*;
    match state {
        GDBStarted => {
            if buttons[0] {
                TryAttachPort {
                    sent_command: 0,
                    host: None,
                }
            } else if buttons[1] {
                AttachFileDialog {
                    sent_command: 0,
                    path: None,
                }
            } else {
                GDBStarted
            }
        }

        AttachFileDialog {
            sent_command: 0,
            ref path,
        } => {
            if buttons[0] {
                AttachFileDialog {
                    sent_command: 1,
                    path: Some(input_fields[0].clone()),
                }
            } else {
                state.clone()
            }
        }

        TryAttachPort {
            sent_command: 0,
            ref host,
        } => {
            if buttons[0] {
                TryAttachPort {
                    sent_command: 1,
                    host: Some(input_fields[0].clone()),
                }
            } else {
                state.clone()
            }
        }

        _ => state,
    }
}
