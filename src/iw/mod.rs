//! IW: logic of the IncorrectWeb Pomoc fork (organization code, signed device API, consent policy).
//! Everything here depends only on `hbb_common`, serde, sha2 and hex, so it is tested by `iw-tests`
//! without Flutter or native libraries. Glue with the rest of RustDesk lives in `src/iw_app.rs`.

pub const IW_VERSION: &str = "1.4.9-iw.2";
pub const DISPLAY_NAME: &str = "IncorrectWeb Pomoc";
pub const SOURCE_URL: &str = "https://github.com/Mattyxs/incorrectweb-pomoc";

pub mod sign;
pub mod org;
pub mod link;
pub mod consent;
pub mod api;
pub mod sessionlog;
pub mod notify;
pub mod scheme;
pub mod policy;
