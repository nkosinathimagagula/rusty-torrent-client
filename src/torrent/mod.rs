use crate::bencode::{value::Parser, BencodeError, BencodeValue};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::from_utf8;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TorrentError {
  #[error("Failed to read torrent file: {0}")]
  Io(#[from] std::io::Error),
  #[error("Bencode parsing failed: {0}")]
  Bencode(#[from] BencodeError),
  #[error("Invalid torrent structure: {0}")]
  InvalidFormat(&'static str),
  #[error("Invalid UTF-8 in string field: {0}")]
  Utf8(#[from] std::str::Utf8Error),
}

#[derive(Debug, Clone)]
pub struct Torrent {
  /// Tracker HTTP/HTTPS URL
  pub announce: String,
  /// Suggested filename to save on disk
  pub name: String,
  /// Total file size in bytes (single-file mode)
  pub length: usize,
  /// Nominal size of each piece in bytes (e.g., 262144 for 256 KB)
  pub piece_length: usize,
  /// Ordered list of 20-byte SHA-1 hashes for each piece
  pub piece_hashes: Vec<[u8; 20]>,
  /// 20-byte SHA-1 hash of the raw bencoded `info` dictionary
  pub info_hash: [u8; 20],
}

impl Torrent{
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, TorrentError> {
    let mut parser = Parser::new(bytes);
    let root = parser.parse()?;

    let root_dict = match root {
      BencodeValue::Dictionary(d) => d,
      _ => return Err(TorrentError::InvalidFormat("Root must be a dictionary")),
    };

    // Extract tracker URL
    let announce_bytes = get_bytes(&root_dict, b"announce")?;
    let announce = from_utf8(announce_bytes)?.to_string();

    // Extract raw 'info' sclice and calculate sha1 info-hash
    let info_span = parser
      .info_span
      .ok_or(TorrentError::InvalidFormat("Missing 'info' dictionary"))?;
    let raw_info_bytes = &bytes[info_span];

    let mut hasher = Sha1::new();
    hasher.update(raw_info_bytes);
    let info_hash: [u8; 20] = hasher.finalize().into();

    // Extract 'info' dictionary fields
    let info_dict = match root_dict.get(&b"info"[..]) {
      Some(BencodeValue::Dictionary(d)) => d,
      _ => return Err(TorrentError::InvalidFormat("info must be a dictionary"))
    };

    let name_bytes = get_bytes(info_dict, b"name")?;
    let name = from_utf8(name_bytes)?.to_string();

    // Note: Multi-file torrents use a `files` list instead of `length`.
    // We enforce single-file mode for now.
    let length = get_int(info_dict, b"length")?
      .try_into()
      .map_err(|_| TorrentError::InvalidFormat("Negative or overflowing file length"))?;

    let piece_length = get_int(info_dict, b"piece length")?
      .try_into()
      .map_err(|_| TorrentError::InvalidFormat("Invalid piece length"))?;

    // Split concatenated 20-byte piece sha1 hashes
    let raw_pieces = get_bytes(info_dict, b"pieces")?;
    let chunks = raw_pieces.chunks_exact(20);

    if !chunks.remainder().is_empty() {
      return Err(TorrentError::InvalidFormat("'pieces' byte length is not a multiple of 20"));
    }

    let piece_hashes: Vec<[u8; 20]> = chunks
      .map(|chunk| {
        let mut hash = [0u8; 20];
        hash.copy_from_slice(chunk);

        hash
      })
      .collect();

    if piece_hashes.is_empty() {
      return Err(TorrentError::InvalidFormat("Torrent contains 0 pieces"));
    }
    
    Ok(Self {
      announce,
      name,
      length,
      piece_length,
      piece_hashes,
      info_hash,
    })
  }

  pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TorrentError> {
    let bytes = std::fs::read(path)?;

    Self::from_bytes(&bytes)
  }

  /// Formats 'info_hash' as a 40-character lowercase hex string for logging
  pub fn info_hash_hex(&self) -> String {
    self.info_hash.iter().map(|b| format!("{:02x}", b)).collect()
  }
}

fn get_bytes<'a>(
  dict: &BTreeMap<&'a [u8], BencodeValue<'a>>,
  key: &[u8]
) -> Result<&'a [u8], TorrentError> {
  match dict.get(key) {
    Some(BencodeValue::ByteString(b)) => Ok(b),
    _ => Err(TorrentError::InvalidFormat("Missing or invalid byte string field"))
  }
}

fn get_int<'a>(
  dict: &BTreeMap<&'a [u8], BencodeValue<'a>>,
  key: &[u8]
) -> Result<i64, TorrentError> {
  match dict.get(key) {
    Some(BencodeValue::Integer(i)) => Ok(*i),
    _ => Err(TorrentError::InvalidFormat("Missing or invalid integer field"))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_synthetic_torrent() {
    // Construct a valid minimal single-file torrent:
    // d
    //   8:announce 23:http://tracker.test/ann
    //   4:info d
    //     6:length i1024e
    //     4:name 8:test.txt
    //     12:piece length i512e
    //     6:pieces 40:<20 bytes of 0xAA><20 bytes of 0xBB>
    //   e
    // e
    let mut raw = Vec::new();
    raw.extend_from_slice(b"d8:announce23:http://tracker.test/ann4:info");

    let info_start = raw.len();
    raw.extend_from_slice(b"d6:lengthi1024e4:name8:test.txt12:piece lengthi512e6:pieces40:");
    raw.extend_from_slice(&[0xAA; 20]);
    raw.extend_from_slice(&[0xBB; 20]);
    raw.extend_from_slice(b"e");
    let info_end = raw.len();

    raw.extend_from_slice(b"e");

    // Compute expected sha-1 independently
    let expected_hash: [u8; 20] = Sha1::digest(&raw[info_start..info_end]).into();

    let torrent = Torrent::from_bytes(&raw).expect("Failed to parse torrent");

    assert_eq!(torrent.announce, "http://tracker.test/ann");
    assert_eq!(torrent.name, "test.txt");
    assert_eq!(torrent.length, 1024);
    assert_eq!(torrent.piece_length, 512);
    assert_eq!(torrent.piece_hashes.len(), 2);
    assert_eq!(torrent.piece_hashes[0], [0xAA; 20]);
    assert_eq!(torrent.piece_hashes[1], [0xBB; 20]);
    assert_eq!(torrent.info_hash, expected_hash);
  }
} 