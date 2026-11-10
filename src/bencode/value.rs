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

    pub fn parse_value(&mut self) -> Result<BencodeValue<'a>, BencodeError> {
        match self.peek()? {
            b'i' => self.parse_integer(),
            b'l' => self.parse_list(),
            b'd' => self.parse_dict(),
            b'0'..=b'9' => self.parse_byte_string().map(BencodeValue::ByteString),
            other => Err(BencodeError::InvalidChar(other as char, self.cursor)),
        }
    }

    pub fn parse(&mut self) -> Result<BencodeValue<'a>, BencodeError> {
        let val = self.parse_value()?;

        if self.cursor != self.input.len() {
            return Err(BencodeError::TrailingData(self.cursor));
        }

        Ok(val)
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
            || (num_slice.starts_with(b"-0"))
            || (num_slice == b"-")
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

    // Byte strings: <len>:<content>
    fn parse_byte_string(&mut self) -> Result<&'a [u8], BencodeError> {
        let start_pos = self.cursor;
        let colon_idx = self.input[self.cursor..]
            .iter()
            .position(|&b| b == b':')
            .map(|pos| self.cursor + pos)
            .ok_or(BencodeError::UnexpectedEof)?;

        let len_slice = &self.input[self.cursor..colon_idx];

        // No leading zeros allowed for length
        if len_slice.len() > 1 && len_slice[0] == b'0' {
            return Err(BencodeError::IvalidInteger(start_pos));
        }

        let len_string = from_utf8(len_slice)
            .map_err(|_| BencodeError::IvalidInteger(start_pos))?;

        let length = len_string
            .parse::<usize>()
            .map_err(|_| BencodeError::IvalidInteger(start_pos))?;

        let content_start = colon_idx + 1;
        let content_end = content_start + length;

        if content_end > self.input.len() {
            return Err(BencodeError::UnexpectedEof);
        }

        self.cursor = content_end;

        Ok(&self.input[content_start..content_end])
    }

    // Lists l<items>e
    fn parse_list(&mut self) -> Result<BencodeValue<'a>, BencodeError> {
        self.expect(b'l')?;
        let mut list = Vec::new();

        while self.peek()? != b'e' {
            list.push(self.parse_value()?);
        }

        self.expect(b'e')?;
        Ok(BencodeValue::List(list))
    }

    // Dictionaries: d<key><value>e
    fn parse_dict(&mut self) -> Result<BencodeValue<'a>, BencodeError> {
        self.expect(b'd')?;
        let mut dict = BTreeMap::new();

        while self.peek()? != b'e' {
            let key_pos = self.cursor;

            // Keys in bencode must be byte strings
            let key = match self.peek()? {
                b'0'..=b'9' => self.parse_byte_string()?,
                _ => return Err(BencodeError::InvalidDictionaryKey(key_pos)),
            };

            let value = self.parse_value()?;
            dict.insert(key, value);
        }

        self.expect(b'e')?;
        Ok(BencodeValue::Dictionary(dict))
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer() {
        let mut p = Parser::new(b"i42e");
        assert_eq!(p.parse().unwrap(), BencodeValue::Integer(42));

        let mut p = Parser::new(b"i-42e");
        assert_eq!(p.parse().unwrap(), BencodeValue::Integer(-42));

        let mut p = Parser::new(b"i0e");
        assert_eq!(p.parse().unwrap(), BencodeValue::Integer(0));

        // Invalid: leading zeros
        assert!(Parser::new(b"i03e").parse().is_err());
        // Invalid: negative zero
        assert!(Parser::new(b"i-0e").parse().is_err());
        // Invalid: just "-"
        assert!(Parser::new(b"i-e").parse().is_err());
    }
}