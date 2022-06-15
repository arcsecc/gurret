use std::collections::HashMap;
/*
 * @TODO:
 * struct Access
{
    filename: Path,
}

impl Metadata for Access
{
    fn execute(&self)
    {
        /*
        let time = // epoch
        update_access_time(self.filename, time)
        */
    }
}*/
use std::path::PathBuf;

use dynamic_exec::Executable;

use crate::metadata::Metadata;


static mut HASHMAP: Option<HashMap<PathBuf, Executable>> = None;

pub struct DynamicMetadata(PathBuf);

impl DynamicMetadata
{
    pub fn new(path: PathBuf) -> Self
    {
        assert_eq!(path.file_name().unwrap(), std::ffi::OsStr::new("execute"));
        let mut buf = path.to_path_buf();
        buf.push("main.rs");
        Self(buf)
    }
}

/*impl Metadata for DynamicMetadata
{
    fn execute(&self) -> std::io::Result<()>
    {
        let map = unsafe
        {
            if HASHMAP.is_none()
            {
                HASHMAP = Some(HashMap::new());
            }
            HASHMAP.as_mut().unwrap()
        };

        if !map.contains_key(&self.0)
        {
            map.insert(self.0.clone(), Executable::from_path(self.0.clone()).unwrap());
        }
        let exec = map.get(&self.0).unwrap();
        //let exec = Executable::from_path(&self.0).unwrap();
        exec.exec_void()
    }
}*/

/*
pub struct DynamicMetadata(Executable);

impl DynamicMetadata
{
    pub fn new(path: PathBuf) -> Self
    {
        assert_eq!(path.file_name().unwrap(), std::ffi::OsStr::new("execute"));
        let mut buf = path.to_path_buf();
        buf.push("main.rs");


        let exec = Executable::from_path(buf).unwrap();

        Self(exec)
    }
}

impl Metadata for DynamicMetadata
{
    fn execute(&self) -> std::io::Result<()>
    {
        self.0.exec_void()
    }
}
*/
