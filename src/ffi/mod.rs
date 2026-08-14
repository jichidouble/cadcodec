//! C ABI for acadrust (LibreDWG-style opaque handles).
//!
//! Consumers compile against the generated `dwg_api.h` and link the
//! `cdylib`/`staticlib` artifact.  The ABI is intentionally small and
//! stable: every exported symbol is `#[no_mangle] extern "C"` and must
//! keep its exact signature.

pub mod data;
pub mod error;
pub mod io;
pub mod types;

pub use data::*;
pub use error::*;
pub use io::*;
pub use types::*;
