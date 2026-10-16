//! C ABI surface for FFI consumers (LibreDWG-style opaque handles).
//!
//! Enable with `--features ffi`. Every exported symbol is `#[no_mangle]`
//! with `extern "C"` and a stable signature; changing them breaks downstream
//! binaries.

mod acis;
mod buffer;
pub mod data;
pub mod dictionary;
pub mod entity;
pub mod entity_create;
pub mod entity_set;
pub mod error;
pub mod io;
pub mod iterate;
mod notification;
pub mod object;
mod remove;
pub mod table;
pub mod tools;
pub mod types;
pub mod util;
pub mod xdata;
pub mod xrecord;

pub use acis::*;
pub use data::*;
pub use dictionary::*;
pub use entity::*;
pub use entity_create::*;
#[allow(unused_imports)]
pub use entity_set::*;
pub use error::*;
pub use io::*;
pub use iterate::*;
pub use notification::*;
pub use object::*;
pub use remove::*;
pub use table::*;
pub use tools::*;
pub use types::*;
pub use xdata::*;
pub use xrecord::*;
