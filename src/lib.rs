//! Tracing utilities
//!
//! # Utilities
//!
//! - [PrettyConsoleLayer](crate::sub::PrettyConsoleLayer): a custom `tracing-subscriber` layer that pretty prints to `stdout`
//!
//! # Features
//!
//! - **subscriber**: activates utilities for `tracing-subscriber`

#[cfg(feature = "subscriber")]
pub mod sub;
