use from_mi_derive::FromMI;

#[derive(Debug, Clone, PartialEq)]
pub enum MIResult {
    Done,
    Running,
    Connected,
    Error { msg: String, code: Option<String> },
    Exit,
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

#[derive(Debug, Clone, PartialEq)]
pub enum AsyncStateStatus {
    Running {
        thread: String,
    },
    Stopped {
        reason: StoppedReason,
        frame: Option<Frame>,
        thread: String,
        stopped_threads: String,
        core: String,
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

/// [docs](https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Frame-Information.html#GDB_002fMI-Frame-Information)
#[derive(Debug, Clone, PartialEq, FromMI)]
#[name = "frame"]
pub struct Frame {
    #[name = "addr"]
    pub addr: u64,
    #[name = "func"]
    pub func: String,
    pub args: Vec<String>,
    pub file: String,
    pub fullname: String,
    pub line: u32,
    pub arch: String,
    /// GDB's docs say this field is present, but I don't see it.
    pub level: Option<String>,
}

/// [docs](https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Breakpoint-Information.html#GDB_002fMI-Breakpoint-Information)
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

//Thread docs
// https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Thread-Information.html#GDB_002fMI-Thread-Information
