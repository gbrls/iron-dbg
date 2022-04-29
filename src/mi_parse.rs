use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_till1, take_while, take_while1};
use nom::character::complete::char;
use nom::multi::{separated_list0, separated_list1};
use nom::sequence::{delimited, pair, terminated};
use nom::IResult;
use std::collections::HashMap;

/// This is how we structure the data that comes from GDB as a String,
/// The string pipeline look as follows:
///
/// (parse the type of the stream and which event happened)
/// then (structure the rest of the string as MIRepr)
/// then ((work on it as MIRepr) OR (convert it to a typed API))
///
///
#[derive(Clone, Debug, PartialEq)]
pub enum MIRepr {
    /// Maybe arrays only hold maps instead of other arrays and maps.
    Array(Vec<MIRepr>),
    Map(HashMap<String, MIRepr>),
    Literal(String),
}

impl MIRepr {
    pub fn to_u32(&self) -> u32 {
        match self {
            MIRepr::Literal(s) => s.parse::<u32>().unwrap(),
            _ => panic!("not number literal"),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            MIRepr::Literal(s) => s.clone(),
            _ => panic!("not literal"),
        }
    }
}
/// This function parses data from GDB such as reason="idk",frame={...}
/// but it doesn't parse the first two tokens that come from GDB
/// such as ^done or *stopped
pub fn mi_repr(input: &str) -> IResult<&str, MIRepr> {
    alt((map, array))(input)
}

fn name(input: &str) -> IResult<&str, String> {
    let (rest, lit) = terminated(
        take_while1(|c: char| c != '=' && (c.is_alphanumeric() || c == '-')),
        char('='),
    )(input)?;

    Ok((rest, lit.into()))
}

fn literal(input: &str) -> IResult<&str, MIRepr> {
    let (rest, lit) = delimited(char('\"'), take_till(|c: char| c == '\"'), char('\"'))(input)?;
    Ok((rest, MIRepr::Literal(lit.into())))
}

fn map(input: &str) -> IResult<&str, MIRepr> {
    use MIRepr::*;

    let (rest, v) = alt((
        separated_list1(char(','), pair(name, alt((map, array, literal)))),
        delimited(
            char('{'),
            separated_list1(char(','), pair(name, alt((map, array, literal)))),
            char('}'),
        ),
    ))(input)?;

    // Sometimes we are not sure if it's a map or an array, if we parse an array as a map we'll lose
    // information, so here we need to fix this

    let mut mp = HashMap::new();
    let mut arr = Vec::new();
    let mut is_array = false;

    for (k, v) in v.into_iter() {
        if mp.contains_key(&k) {
            is_array = true;
        }

        mp.insert(k.clone(), v.clone());

        let mut kv = HashMap::new();
        kv.insert(k, v);
        arr.push(Map(kv));
    }

    if is_array {
        Ok((rest, Array(arr)))
    } else {
        Ok((rest, Map(mp)))
    }
}

fn array(input: &str) -> IResult<&str, MIRepr> {
    let (rest, vals) = delimited(
        char('['),
        separated_list0(char(','), alt((array, literal, map))),
        char(']'),
    )(input)?;

    Ok((rest, MIRepr::Array(vals)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_literal() {
        let (rest, s) = name(r#"reason="breakpoint-hit""#).unwrap();
        assert_eq!(&s, "reason");

        let (rest, v) = literal(rest).unwrap();
        assert_eq!(v, MIRepr::Literal("breakpoint-hit".into()));
    }

    #[test]
    fn test_map_array() {
        let (rest, v) = map(r#"reason="breakpoint-hit",line="4""#).unwrap();
        println!("{v:?}");
        let (rest, v) = map(r#"brkpt={reason="breakpoint-hit",line="4"}"#).unwrap();
        println!("{v:?}");
        let (rest, v) = map(r#"brkpt=["first","second","third"]"#).unwrap();
        println!("{v:?}");
        let (rest, v) =
            map(r#"brkpt=[{reason="breakpoint-hit",line="4"},{reason="breakpoint-hit",line="8"}]"#)
                .unwrap();
        println!("{v:?}");

        let (rest, v) = map(r#"thread-id="all""#).unwrap();
        println!("{v:?}, rest {rest}");
    }

    #[test]
    fn test_stack_list_frames() {
        let s = r#"stack=[frame={level="0",addr="0x000000000040114f",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="8",arch="i386:x86-64"},frame={level="1",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="2",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="3",addr="0x0000000000401167",func="fib",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="10",arch="i386:x86-64"},frame={level="4",addr="0x000000000040119a",func="main",file="example.c",fullname="/home/gbrls/Programming/iron-dbg/res/example.c",line="15",arch="i386:x86-64"}]"#;
        let (_, v) = mi_repr(s).unwrap();
        println!("{v:#?}");
    }
}
