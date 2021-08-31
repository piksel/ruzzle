#[macro_use]
extern crate lazy_static;

use core::result;
use std::error::Error;

pub mod paths;
pub mod tetrominos;
pub mod engine;
pub mod config;

pub type Result<T> = result::Result<T, Box<dyn Error>>;