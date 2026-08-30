use nom::{IResult, Parser as _, bytes::complete::tag};

// https://www.rfc-editor.org/info/rfc9110/#section-5.6.3
pub fn ows_rfc9110(input: &str) -> IResult<&str, ()> {
    nom::combinator::recognize(nom::multi::many0(nom::branch::alt((tag(" "), tag("\t")))))
        .parse(input)
        .map(|(next_input, _)| (next_input, ()))
}
