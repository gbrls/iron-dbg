use crate::mi;
use crate::mi_parse::MIRepr;
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

    get(&repr, line_path).and_then(|x| {
        if let MIRepr::Literal(s) = x {
            Some(s.parse::<u32>().unwrap())
        } else {
            panic!("Expected lines literal");
        }
    })
}

pub fn current_file(input: &mi::Output) -> Option<PathBuf> {
    let repr = mi_repr(input);
    if repr.is_none() {
        return None;
    }

    let repr = repr.unwrap();

    let line_path = &["frame", "fullname"];

    get(&repr, line_path).and_then(|x| {
        if let MIRepr::Literal(s) = x {
            Some(PathBuf::from(&s))
        } else {
            panic!("Expected lines literal");
        }
    })
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
}
