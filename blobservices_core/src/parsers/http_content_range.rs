#[derive(PartialEq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub entire_size: u64,
}

impl ContentRange {
    pub fn size(&self) -> u64 {
        (self.end - self.start) + 1
    }
}

impl std::fmt::Display for ContentRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.start <= self.end);
        assert!(self.end < self.entire_size);
        write!(f, "bytes {}-{}/{}", self.start, self.end, self.entire_size)
    }
}
