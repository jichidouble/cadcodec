//! C ABI surface for FFI consumers (LibreDWG-style opaque handles).
//!
//! Enable with `--features ffi`. Every exported symbol is `#[no_mangle]`
//! with `extern "C"` and a stable signature; changing them breaks downstream
//! binaries.

pub mod types;
pub mod error;
pub mod tools;
pub mod data;
pub mod io;
pub mod object;
pub mod iterate;

pub use types::*;
pub use error::*;
pub use tools::*;
pub use data::*;
pub use io::*;
pub use object::*;
pub use iterate::*;
