#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod application;
mod auth;
mod cookie;
mod domain;
mod error;
mod http;
mod infrastructure;
mod runtime;
mod state;
mod util;
mod workers;

pub mod config;

pub use config::{Config, configured_path};
pub use runtime::run;
