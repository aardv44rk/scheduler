//! # Task Scheduler
//!
//! A Rust-based, persistent task scheduler built with Axum, SQLx, and Tokio.
pub mod api;
pub mod config;
pub mod db;
pub mod domain;
pub mod errors;
pub mod scheduler;
pub mod service;
pub mod tests;
