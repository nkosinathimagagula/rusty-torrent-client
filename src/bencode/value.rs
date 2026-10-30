use std::collections::BTreeMap;
use std::str::from_utf8;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum BencodeError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Invalid character '{0}' at offset {1}")]
    InvalidChar(char, usize),
    #[error("Integer parse error at offset {0}")]
    IvalidInteger(usize),
    #[error("Dictionary keys must be byte strings at offset {0}")]
    InvalidDictionaryKey(usize),
    #[error("Trailing unparsed bytes starting at offset {0}")]
    TrailingData(usize),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BencodeValue<'a> {
    Integer(i64),
    ByteString(&'a [u8]),
    List(Vec<BencodeValue<'a>>),
    Dictionary(BTreeMap<&'a [u8], BencodeValue<'a>>),
}

pub struct Parser<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    fn peek(&self) -> Result<u8, BencodeError> {
        self.input
            .get(self.cursor)
            .copied()
            .ok_or(BencodeError::UnexpectedEof)
    }

    fn advance(&mut self) -> Result<u8, BencodeError> {
        let byte = self.peek()?;
        self.cursor += 1;
        Ok(byte)
    }

    fn expect(&mut self, expected: u8) -> Result<(), BencodeError> {
        let pos = self.cursor;
        let actual = self.advance()?;

        if actual == expected {
            Ok(())
        } else {
            Err(BencodeError::InvalidChar(actual as char, pos))
        }
    }

    // Integers: i<number>e
    fn parse_integer(&mut self) -> Result<BencodeValue<'a>, BencodeError> {
        let start_pos = self.cursor;
        self.expect(b'i')?;

        let end_idx = self.input[self.cursor..]
            .iter()
            .position(|&b| b == b'e')
            .map(|pos| self.cursor + pos)
            .ok_or(BencodeError::UnexpectedEof)?;

        let num_slice = &self.input[self.cursor..end_idx];

        if num_slice.is_empty() {
            return Err(BencodeError::IvalidInteger(start_pos));
        }

        // Validate Bencode edge cases: no "-0", leading zeros forbidden
        // (e.g. "i03e" is invalid)
        if (num_slice.len() > 1 && num_slice[0] == b'0')
            || (num_slice.len() > 2 && num_slice[0] == b'-' && num_slice[1] == b'0')
            || (num_slice.len() == 1 && num_slice[0] == b'-')
        {
            return Err(BencodeError::IvalidInteger(start_pos));
        }

        let num_string =
            from_utf8(num_slice).map_err(|_| BencodeError::IvalidInteger(start_pos))?;

        let val = num_string
            .parse::<i64>()
            .map_err(|_| BencodeError::IvalidInteger(start_pos))?;

        // skip past 'e'
        self.cursor = end_idx + 1;
        Ok(BencodeValue::Integer(val))
    }
}
