use nom::{
    IResult, Parser as _,
    bytes::complete::{tag, tag_no_case},
    character::complete::u64,
    combinator::all_consuming,
    sequence::{delimited, preceded, separated_pair},
};

use crate::parsers::rfc::ows_rfc9110;

#[derive(Debug, PartialEq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub entire_size: u64,
}

impl ContentRange {
    pub fn size(&self) -> u64 {
        (self.end - self.start) + 1
    }

    pub fn parse(input: &str) -> IResult<&str, ContentRange> {
        all_consuming(delimited(
            preceded(tag_no_case("bytes"), ows_rfc9110),
            separated_pair(separated_pair(u64, tag("-"), u64), tag("/"), u64).map(|x| {
                ContentRange {
                    start: x.0.0,
                    end: x.0.1,
                    entire_size: x.1,
                }
            }),
            ows_rfc9110,
        ))
        .parse(input)
    }
}

impl std::fmt::Display for ContentRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.start <= self.end);
        assert!(self.end < self.entire_size);
        write!(f, "bytes {}-{}/{}", self.start, self.end, self.entire_size)
    }
}

#[cfg(test)]
mod tests {
    use super::ContentRange;

    #[test]
    fn parse() {
        let test_cases = [
            (
                "bytes 0-1/2",
                Some(ContentRange {
                    start: 0,
                    end: 1,
                    entire_size: 2,
                }),
            ),
            (
                "bytes 123-456/789",
                Some(ContentRange {
                    start: 123,
                    end: 456,
                    entire_size: 789,
                }),
            ),
        ];

        for (input, expected) in test_cases {
            let result = ContentRange::parse(input).ok().map(|x| x.1);
            assert_eq!(expected, result);
        }
    }
}
