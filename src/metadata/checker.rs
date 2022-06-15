use std::path::Path;

use dynamic_exec::Executable;

use crate::metadata::{Metadata, Operation};

pub fn get_metadata_checker<P: AsRef<Path>>(path: P) -> Box<dyn Metadata<Item = ()>>
{
    Box::new(RustChecker::new(path.as_ref()))
}


/*
pub struct RustChecker(PathBuf);

impl RustChecker
{
    // Expect the 'check' folder
    pub fn new(path: &Path) -> Self
    {
        assert_eq!(path.file_name().unwrap(), std::ffi::OsStr::new("check"));
        let mut buf = path.to_path_buf();
        buf.push("main.rs");

        assert!(buf.exists());

        Self(buf)
    }
}

impl MetadataChecker for RustChecker
{
    fn check(&self, op: Operation) -> bool
    {
        let exec = Executable::from_path(&self.0).unwrap();
        exec.exec::<bool>(Some(op.to_string())).unwrap()
    }
}*/


pub struct RustChecker(Executable, Executable);

impl RustChecker
{
    // Expect the 'check' folder
    pub fn new(path: &Path) -> Self
    {
        assert_eq!(path.file_name().unwrap(), std::ffi::OsStr::new("check"));
        let mut buf = path.to_path_buf();
        buf.push("main.rs");

        assert!(buf.exists());
        let check = Executable::from_path(buf.clone()).unwrap();


        buf.pop();
        buf.pop();
        buf.push("update");
        buf.push("main.rs");

        let update = Executable::from_path(buf).unwrap();

        Self(check, update)
    }
}

impl Metadata for RustChecker
{
    type Item = ();

    fn check(&self, op: Operation) -> bool
    {
        self.0.exec::<bool>(Some(op.to_string())).unwrap()
    }

    fn update(&self) -> std::io::Result<()>
    {
        self.1.exec_void().unwrap();
        Ok(())
    }

    fn access(&self) -> std::io::Result<Self::Item>
    {
        todo!()
    }
}
