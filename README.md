# Rusty Torrent Client

Rusty Torrent Client is a learning project for exploring Rust by building a BitTorrent client from the ground up. The goal is to implement the protocol in small, understandable steps—from reading torrent metadata to downloading and verifying pieces—while learning how Rust handles systems programming and asynchronous networking.

> **Project status:** This project is at an early stage. The features below are development goals, not a description of functionality currently available.

## Learning goals

Building this project is an opportunity to learn about:

- Rust ownership, borrowing, lifetimes, enums, error handling, and module organization.
- Binary data parsing and designing a custom Bencode decoder.
- SHA-1 hashing and verifying downloaded data against torrent metadata.
- HTTP tracker requests, TCP connections, and the BitTorrent handshake.
- Designing and implementing a binary network protocol.
- Asynchronous programming with Tokio, tasks, and channels.
- Coordinating concurrent downloads from multiple peers.
- File I/O, testing, debugging, and building a command-line application.

## Planned features

The project is intended to grow through the following milestones:

1. Parse `.torrent` files with a custom Bencode decoder.
2. Calculate the 20-byte SHA-1 `info_hash` from the exact encoded `info` dictionary.
3. Query an HTTP tracker and parse its response to discover peers.
4. Connect to peers over TCP and complete the 68-byte BitTorrent handshake.
5. Implement the peer wire protocol, including bitfields, choke/unchoke, interested, and block request messages.
6. Download blocks of up to 16 KiB, assemble pieces, verify each piece's SHA-1 hash, and write verified data to disk.
7. Download pieces concurrently from multiple peers using Tokio and asynchronous channels.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) with Cargo.

### Build and run

Clone the repository, then run the current starter application:

```sh
git clone https://github.com/nkosinathimagagula/rusty-torrent-client.git
cd rusty-torrent-client
cargo run
```

At this early stage, the application is only a Rust starter program. Torrent parsing and downloading are planned and are not yet available.

## Development

Build the project and run its tests with Cargo:

```sh
cargo build
cargo test
```

As features are added, tests should cover malformed torrent data, protocol message encoding and decoding, piece hash verification, and other edge cases.

## Responsible use

This project is for learning and experimentation. Only use it with content you are authorized to download and share, and follow the laws and network rules that apply where you are. Avoid testing against peers or trackers without permission.

## License
