//! History storage adapters

pub mod qdrant;

pub use qdrant::QdrantHistoryStorage;

#[cfg(test)]
mod tests;
