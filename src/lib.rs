pub mod config;
pub mod file_system;
pub mod lattice;
pub mod metadata;
pub mod mount;
pub mod permission;
pub mod policy;
pub mod socket;
pub mod table;

#[macro_use] extern crate shell;

pub use std::{
    ffi::OsStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub use config::*;
pub use file_system::XmpFS;
pub use lazy_static::lazy_static;
lazy_static! {
    pub static ref BASE_PATH: String = config("TARGET");
}

#[macro_export]
macro_rules! TABLE {
    ($table:expr) => {
        $table.lock().expect("getting lock")
    };
}

pub fn mount_file_system()
{
    let mountpoint = config("PATH");
    let _tmp_mountpoint = config("TARGET");
    let _file = config("FILE");
    //mount::mount(&_file, &_tmp_mountpoint);

    /*let options = [
        "-o",
        "rw,default_permissions",
        "-o",
        "fsname=xmp",
        "-o",
        "nonempty",
        "-o",
        "auto_unmount",
        "-o",
        "allow_other",
    ]
    .iter()
    .map(|o| o.as_ref())
    .collect::<Vec<&OsStr>>();*/

    use fuser::MountOption;
    let options = vec![
        MountOption::FSName("xmp".to_string()),
        MountOption::NoAtime,
        MountOption::Async,
        //MountOption::CUSTOM("big_writes".into()),
        MountOption::AutoUnmount,
        MountOption::AllowOther,
    ];

    let mut xmp = XmpFS::new();
    xmp.populate_root_dir();

    let state = Arc::clone(&xmp.table);
    let fs_handle = fuser::spawn_mount2(xmp, mountpoint, &options).unwrap();


    // Exit condition
    let term = Arc::new(AtomicBool::new(false));
    // capture ctrl + c and set exit condition to true in the case
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term)).unwrap();


    let t2 = Arc::clone(&term);
    let thread_handle = std::thread::spawn(move || {
        socket::spawn(t2, state);
    });

    while !term.load(Ordering::Relaxed)
    {}
    drop(fs_handle);
    //mount::umount(&_tmp_mountpoint);

    let _ = thread_handle.join();
}
