//! Web interface module
//!
//! Provides an HTTP server for browser-based access to FerrumDB.
//! This allows users to interact with the database through a web UI.

mod server;
mod handlers;

pub use server::{run_web_server, run_web_with_cluster};
