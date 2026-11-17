use crate::bencode::{value::Parser, BencodeError, BencodeValue};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::Path;
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
  pub piece_hash: Vec<[u8; 20]>,
  /// 20-byte SHA-1 hash of the raw bencoded `info` dictionary
  pub info_hash: [u8; 20],
}