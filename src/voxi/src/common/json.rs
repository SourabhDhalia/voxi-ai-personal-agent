//! JSON module — re-exports serde_json for convenience.
//!
//! This replaces the hand-written JSON parser with serde_json.

pub use serde::{Deserialize, Serialize};
pub use serde_json::Value as JsonValue;
pub use serde_json::{from_str as parse_json, json, to_string, to_string_pretty, Map, Number};
