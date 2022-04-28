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

    let mut mp = HashMap::new();

    for (k, v) in v.into_iter() {
        mp.insert(k, v);
    }

    Ok((rest, Map(mp)))
}

fn array(input: &str) -> IResult<&str, MIRepr> {
    let (rest, vals) = delimited(
        char('['),
        separated_list0(char(','), alt((map, array, literal))),
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
}
