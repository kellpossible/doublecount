//! A double entry accounting system/library.
//! 
//! The doublecount package has the following optional cargo features:
//! 
//! # Optional Features
//! 
//! + `serde-support`
//!   + Disabled by default
//!   + Enables support for serialization/de-serialization via `serde`
//!   + Enables support for json serialization/de-serialization via `serde_json`

extern crate chrono;
extern crate nanoid;
extern crate rust_decimal;
extern crate commodity;
extern crate thiserror;

#[cfg(feature = "serde-support")]
extern crate serde;
#[cfg(feature = "serde-support")]
extern crate serde_json;

pub mod accounting;

#[doc(no_inline)] pub use accounting::*;
