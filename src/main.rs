/*
#[macro_export]
macro_rules! TABLE {
    ($table:expr) => {
        $table.lock().expect("getting lock")
    };
}

#[macro_use] extern crate shell;

use std::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;

lazy_static! {
    static ref BASE_PATH: String = config("TARGET");
}*/


//mod byte_map;
/*mod config;
mod file_system;
mod lattice;
mod mount;
mod permission;
mod policy;
mod socket;
mod table;

use std::{ffi::OsStr, sync::Arc};

use config::*;
use file_system::XmpFS;
*/


pub use lh_mount::*;

/*#[macro_export]
macro_rules! TABLE {
    ($table:expr) => {
        $table.lock().expect("getting lock")
    };
}*/


fn checkout(name: &str)
{
    let mut table = table::Table::from_file().unwrap_or_else(|_| table::Table::default());
    table.insert(format!("{}/{}", *BASE_PATH, name)).expect("inserting");
    table.flush().expect("flushing table");
}

fn main()
{
    let args: Vec<_> = std::env::args().collect();
    let args: Vec<_> = args.iter().map(String::as_str).collect();

    match args.as_slice()
    {
        &[_, "checkout", name] =>
        {
            checkout(name);
        },
        _ => mount_file_system(),
    }
}
