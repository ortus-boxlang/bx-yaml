//! The MatchBox backend for the bx-yaml module: the same three functions as the JVM module, without a JVM.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

pub mod model;
mod plugin;
pub mod read;
pub mod timestamp;
pub mod write;
pub mod zone;
