//! Shared integration-test harness.
//!
//! Helpers here are used across suites; not every helper is exercised by every
//! suite, so dead-code warnings are allowed at the harness level only.
#![allow(dead_code)]

pub mod assertions;
pub mod client;
pub mod db;
pub mod fixtures;
pub mod harness;

pub use assertions::{assert_error, assert_status};
pub use client::Auth;
pub use harness::{app, TestApp};
