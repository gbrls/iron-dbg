use crate::mi_parse::MIRepr;
use crate::{mi, mi_types};
use std::path::{Path, PathBuf};

/// We are using a data oriented programming approach here,
/// instead of converting the MIRepr to a typed interface
/// we are using it as it its, maybe this will be changed
/// in the future.
pub fn get(data: &MIRepr, query: &[&str]) -> Option<MIRepr> {
    let mut cur = data;
    for dir in query {
        match cur {
            MIRepr::Map(mp) => match mp.get(*dir) {
                Some(x) => cur = x,
                _ => return None,
            },
            _ => return None,
        }
        println!("[Query] {cur:?}");
    }
    Some(cur.clone())
}

fn mi_repr(input: &mi::Output) -> Option<MIRepr> {
    match input {
        mi::Output::ExecAsync(_, r) => Some(r.clone()),
        mi::Output::NotifyAsync(_, r) => Some(r.clone()),
        mi::Output::ResultRecord(_, Some(r)) => Some(r.clone()),
        _ => None,
    }
}

pub fn current_line(input: &mi::Output) -> Option<u32> {
    let repr = mi_repr(input);
    if repr.is_none() {
        return None;
    }

    let repr = repr.unwrap();

    let line_path = &["frame", "line"];

    get(&repr, line_path).and_then(|x| Some(x.to_u32()))
}

pub fn current_file(input: &mi::Output) -> Option<PathBuf> {
    let repr = mi_repr(input);
    if repr.is_none() {
        return None;
    }

    let repr = repr.unwrap();

    let line_path = &["frame", "fullname"];

    get(&repr, line_path).and_then(|x| Some(PathBuf::from(x.to_string())))
}

pub fn frames_from_repr(repr: &MIRepr) -> Option<Vec<mi_types::Frame>> {
    get(&repr, &["stack"]).and_then(|repr| match repr {
        MIRepr::Array(v) => match &v[0] {
            MIRepr::Array(v) => Some(
                v.into_iter()
                    .map(|x| frame_from_repr(&x).unwrap())
                    .collect(),
            ),
            _ => None,
        },
        _ => None,
    })
}

pub fn frames(input: &mi::Output) -> Option<Vec<mi_types::Frame>> {
    let repr = mi_repr(input);
    if repr.is_none() {
        return None;
    }

    let repr = repr.unwrap();
    frames_from_repr(&repr)
}

/// Querying the output of -stack-list-frames
fn frame_from_repr(repr: &MIRepr) -> Option<mi_types::Frame> {
    get(&repr, &["frame"]).and_then(|frame| {
        let func = get(&frame, &["func"]).unwrap().to_string();
        let level = get(&frame, &["level"]).unwrap().to_u32();

        Some(mi_types::Frame {
            func,
            level,
            args: None,
        })
    })
}

/// Querying the output of -stack-list-arguments 2
pub fn frame_args_from_repr(repr: &MIRepr) -> Option<Vec<(String, String, String)>> {
    get(&repr, &["stack-args"]).and_then(|frames| {
        match frames {
            MIRepr::Array(f) => None,
            _ => None,
        }
    })
}

pub fn frame(input: &mi::Output) -> Option<mi_types::Frame> {
    let repr = mi_repr(input);
    if repr.is_none() {
        return None;
    }

    let repr = repr.unwrap();

    frame_from_repr(&repr)
}

pub fn has_exited(input: &mi::Output) -> bool {
    match input {
        mi::Output::ExecAsync(state, repr) => match get(repr, &["reason"]) {
            Some(r) => {
                if let MIRepr::Literal(s) = r {
                    s == "exited-normally"
                } else {
                    false
                }
            }
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mi_parse;

    #[test]
    fn test_get() {
        let v = mi_parse::mi_repr(r#"brkpt={reason="breakpoint-hit",line="4"}"#)
            .unwrap()
            .1;
        assert_eq!(
            get(&v, &["brkpt", "line"]),
            Some(MIRepr::Literal(String::from("4")))
        );
    }

    #[test]
    fn test_frames() {
        let v = mi_parse::mi_repr(
            r#"stack=[frame={level="0",addr="0x000000000040115a",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="1",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="2",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="3",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="4",addr="0x000000000040119a",func="main",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="15",arch="i386:x86-64"}]"#,
        ).unwrap().1;

        println!("{:?}", frames_from_repr(&v));
    }

    #[test]
    fn test_frame_args() {
        let v = mi_parse::mi_repr(r#"stack-args=[frame={level="0",args=[{name="a",type="int",value="1"}]},frame={level="1",args=[{name="a",type="int",value="2"}]},frame={level="2",args=[{name="a",type="int",value="3"}]},frame={level="3",args=[{name="a",type="int",value="4"}]},frame={level="4",args=[{name="a",type="int",value="5"}]},frame={level="5",args=[]}]"#)
            .unwrap()
            .1;

        println!("{v:#?}");
    }
}
