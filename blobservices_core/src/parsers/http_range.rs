use std::{cmp::min, num::NonZeroU64};

use nom::{
    IResult, Parser as _,
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::u64,
    combinator::{all_consuming, opt},
    sequence::{delimited, preceded, separated_pair},
};

use crate::parsers::rfc::ows_rfc9110;

pub enum BytesRange {
    IntRange(u64, Option<u64>),
    SuffixRange(u64),
}

pub struct NormalizedBytesRange {
    pub start: u64,
    pub end: u64,
}

impl BytesRange {
    /// None → invalid
    pub fn normalize(&self, entire_size: NonZeroU64) -> Option<NormalizedBytesRange> {
        let entire_size = entire_size.into();
        match self {
            BytesRange::IntRange(start, end) => {
                if *start >= entire_size {
                    return None;
                }
                if let Some(end) = end {
                    if *end < *start {
                        return None;
                    }
                }
                Some(NormalizedBytesRange {
                    start: *start,
                    end: end
                        .map(|x| min(x, entire_size - 1))
                        .unwrap_or(entire_size - 1),
                })
            }
            BytesRange::SuffixRange(start) => {
                if *start == 0 {
                    return None;
                }
                if *start >= entire_size {
                    return Some(NormalizedBytesRange {
                        start: 0,
                        end: entire_size - 1,
                    });
                }
                Some(NormalizedBytesRange {
                    start: entire_size - start,
                    end: entire_size - 1,
                })
            }
        }
    }
}

// https://www.rfc-editor.org/info/rfc9110/#section-14.1.1 の退化版
// 複数range・bytes以外は対応しない
pub fn bytes_range_specifier(input: &str) -> IResult<&str, BytesRange> {
    all_consuming(delimited(
        preceded(
            tag_no_case("bytes"),
            delimited(ows_rfc9110, tag("="), ows_rfc9110),
        ),
        alt((
            // 1-2 とか 3-
            separated_pair(u64, delimited(ows_rfc9110, tag("-"), ows_rfc9110), opt(u64))
                .map(|x| BytesRange::IntRange(x.0, x.1)),
            // -3- (最後から3byte)
            preceded(tag("-"), u64).map(|x| BytesRange::SuffixRange(x)),
        )),
        ows_rfc9110,
    ))
    .parse(input)
}
