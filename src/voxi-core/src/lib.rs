//! voxi-core — Unified Plugin SDK & Core framework
//!
//! Provides C FFI functions for:
//! - LLM data type handles (messages, tools, responses)
//! - HTTP helper (curl-like API backed by ureq)
//!   And internal framework logic for plugins.

#![allow(unused)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::vec_box)]

pub mod llm_types;
pub mod curl_wrapper;

pub mod framework;
