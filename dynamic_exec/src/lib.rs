use std::{
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

pub struct Executable(PathBuf);

impl Executable
{
    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self>
    {
        Ok(Self(compile_file(path)?))
    }

    pub fn exec<T>(&self, arg: Option<String>) -> std::io::Result<T>
    where
        T: FromStr,
    {
        run_file::<T, _>(&self.0, arg)
    }

    pub fn exec_void(&self) -> std::io::Result<()>
    {
        run_file_void(&self.0)
    }
}

impl Drop for Executable
{
    fn drop(&mut self)
    {
        std::fs::remove_file(&self.0).unwrap();
    }
}


pub fn compile_file<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf>
{
    let path = path.as_ref();

    let parent = path.parent().unwrap();
    let file_name = path.file_name().unwrap();

    Command::new("rustc")
        .current_dir(parent)
        .arg(file_name)
        .output()
        .and_then(|output| {
            if output.status.success()
            {
                // I want to _just_ do canonicalize, but tempfile screws us over
                let stem = path.file_stem().expect("getting file_stem");

                let mut buf = PathBuf::new();
                buf.push(parent);
                buf.push(stem);

                Ok(buf)
            }
            else
            {
                Err(std::io::Error::from_raw_os_error(output.status.code().unwrap()))
            }
        })
}

pub fn run_file_void<P>(path: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    Command::new(path.as_os_str()).output().map(|_| ())
}

pub fn run_file<T, P>(path: P, arg: Option<String>) -> std::io::Result<T>
where
    P: AsRef<Path>,
    T: FromStr,
{
    let path = path.as_ref();
    let mut command = Command::new(path.as_os_str());
    if let Some(arg) = arg
    {
        command.arg(arg);
    }

    command.output().and_then(|output| {
        if output.status.success()
        {
            const EINVAL: i32 = 22;
            let s = std::str::from_utf8(&output.stdout).unwrap().trim();
            s.parse::<T>().map_err(|_| std::io::Error::from_raw_os_error(EINVAL))
        }
        else
        {
            Err(std::io::Error::from_raw_os_error(output.status.code().unwrap()))
        }
    })
}


#[cfg(test)]
mod tests
{
    use std::{ffi::OsString, io::Write};

    use tempfile::Builder;

    use super::*;


    struct Guard(OsString);
    impl Guard
    {
        fn new(path: impl AsRef<Path>) -> Self
        {
            Self(path.as_ref().file_stem().unwrap().to_os_string())
        }
    }
    impl Drop for Guard
    {
        fn drop(&mut self)
        {
            //std::fs::remove_file(&self.0).unwrap();
        }
    }


    #[test]
    fn creates_file()
    {
        let file = Builder::new().prefix("temp_").suffix(".rs").rand_bytes(5).tempfile().unwrap();

        writeln!(file.as_file(), "fn main(){{}}").unwrap();
        let _guard = Guard::new(file.path());

        assert!(compile_file(file.path()).is_ok());
    }

    #[test]
    fn execute_path()
    {
        let file = Builder::new().prefix("temp_").suffix(".rs").rand_bytes(5).tempfile().unwrap();
        writeln!(file.as_file(), "fn main(){{println!(\"5\")}}").unwrap();
        let _guard = Guard::new(file.path());
        let handle = compile_file(file.path()).unwrap();
        assert_eq!(run_file::<isize, _>(handle, None).unwrap(), 5);
    }

    #[test]
    fn test_interface()
    {
        let file = Builder::new().prefix("temp_").suffix(".rs").rand_bytes(5).tempfile().unwrap();
        writeln!(file.as_file(), "fn main(){{println!(\"12\")}}").unwrap();

        let exec = Executable::from_path(file.path()).unwrap();
        assert_eq!(exec.exec::<i32>(None).unwrap(), 12);
    }

    #[test]
    fn test_execute_with_arg()
    {
        let file = Builder::new().prefix("temp_").suffix(".rs").rand_bytes(5).tempfile().unwrap();
        writeln!(
            file.as_file(),
            "fn main(){{
            let arg = std::env::args().nth(1).unwrap();
            println!(\"{{arg}}\");
        }}"
        )
        .unwrap();

        let arg = String::from("foo");
        let exec = Executable::from_path(file.path()).unwrap();
        assert_eq!(exec.exec::<String>(Some(arg.clone())).unwrap(), arg);
    }
}
