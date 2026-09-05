//! # z3s-storage
//!
//! Engine de armazenamento local (Extent Store / ShardStore com suporte a I/O assíncrono, WAL e Bitrot Scrubber).

pub mod engine;
pub mod extent;
pub mod index;
pub mod scrubber;
pub mod wal;

pub use engine::{StorageEngine, StorageError, DEFAULT_EXTENT_CAPACITY};
pub use extent::{BlockHeader, ExtentError, ExtentFile, ExtentHeader, FormatError};
pub use index::{ShardLocation, StorageIndex};
pub use scrubber::{BitrotScrubber, ScrubReport};
pub use wal::{WalError, WalRecord, WriteAheadLog};
