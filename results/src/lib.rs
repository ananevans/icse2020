#![feature(extern_prelude)]

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate chrono;

pub mod implicit;
pub mod functions;
pub mod unsafety_sources;
pub mod blocks;

mod util;

