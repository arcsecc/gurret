use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::File,
    io::{prelude::*, BufRead, BufReader, BufWriter},
    path::{Path, PathBuf},
};

use crate::{file_system::Program, permission::*, XmpFS, BASE_PATH, TABLE};

pub fn config<S: AsRef<str>>(name: S) -> String
{
    let file = match std::fs::File::open("/home/sivert/master/lh_mount/config")
    {
        Ok(f) => f,
        Err(_) => std::fs::File::open("../config").expect("could not find config file"),
    };

    std::io::BufReader::new(file)
        .lines()
        .map(|line| {
            line.unwrap()
                .split_once("=")
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .unwrap()
        })
        .collect::<HashMap<String, String>>()
        .remove(name.as_ref())
        .unwrap()
}


#[allow(dead_code)]
pub fn read_file_content<S: AsRef<str>>(filename: S) -> Vec<u8>
{
    let file = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .open(filename.as_ref())
        .unwrap();
    let mut buf_reader = std::io::BufReader::new(file);
    let mut content = Vec::new();
    buf_reader.read_to_end(&mut content).expect("reading file");
    content
}

#[allow(dead_code)]
pub fn write_file_content<S: AsRef<str>>(filename: S, content: &[u8])
{
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(filename.as_ref())
        .unwrap();

    let mut buf_writer = BufWriter::new(file);
    buf_writer.write(content).unwrap();
}


pub fn log_time() -> u64
{
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
/*pub fn log_operation<P, C>(path: P, prefix: C, content: C)
where
    P: AsRef<std::path::Path>,
    C: AsRef<str>,
{
    let mut filename = std::path::PathBuf::from(path.as_ref());
    let s = filename.file_name().unwrap();
    filename.set_file_name(&format!(".{}", s.to_str().unwrap()));

    println!("filename: ::: {:?}", filename.as_os_str());
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename.as_path())
        .expect("opening file");
    println!("???");

    // Another allocation, bleh
    let content = format!("{} {} {}\n", prefix.as_ref(), content.as_ref(), log_time());
    file.write_all(content.as_bytes()).unwrap();
}*/

pub fn get_name_and_file<'s>(info: &'s str) -> (&'s str, &'s str)
{
    let s = info.split_once(char::from(0)).unwrap();

    let path = std::path::Path::new(s.0);
    let name = path.file_name().unwrap().to_str().unwrap();
    (name, s.1.trim_matches(char::from(0)))
}

pub fn get_temp_name(path: impl AsRef<Path>) -> String
{
    let filename = std::path::PathBuf::from(path.as_ref());
    let s = filename.file_name().unwrap();
    format!(".{}", s.to_str().unwrap())
}

pub fn file_for_temp(path: &str) -> bool
{
    let path = Path::new(path);
    let file_name = path.file_name().unwrap();
    let s = file_name.to_str().unwrap();
    (s[1..]).starts_with('.')
}

#[allow(dead_code)]
pub fn get_program_name(req: &fuser::Request) -> Option<OsString>
{
    let cmdline = match std::fs::read_to_string(format!("/proc/{}/cmdline", req.pid()))
    {
        Ok(s) => s,
        _ => return None,
    };
    let s = cmdline.as_str();

    let path = |st: &str| std::path::Path::new(st).file_name().map(OsStr::to_os_string);

    let s = match s.find(char::from(0))
    {
        Some(idx) => &s[..idx],
        _ => &s,
    };
    path(s)
}

pub fn get_cmdline(req: &fuser::Request) -> Option<String>
{
    std::fs::read_to_string(format!("/proc/{}/cmdline", req.pid())).ok()
}

pub fn get_cmdline_output(req: &fuser::Request) -> Option<Vec<String>>
{
    get_cmdline(req)
        .map(|s| s.split(char::from(0)).map(str::to_string).collect())
        .map(|mut v: Vec<String>| {
            v.pop();
            v
        })
}

pub fn get_cmdline_output_and_name(req: &fuser::Request) -> Option<(String, String)>
{
    std::fs::read_to_string(format!("/proc/{}/cmdline", req.pid())).ok().map(|s| {
        (
            s.split_once(char::from(0)).expect("could not spilt once").0.to_string(),
            s.replace(char::from(0), " "),
        )
    })
}

pub fn program_name_from_path<P: AsRef<Path>>(program_name: P) -> OsString
{
    let mut pathbuf = std::path::PathBuf::new();
    pathbuf.push(&program_name);
    let s = pathbuf.file_name().expect("no file name").to_os_string();
    s
}

pub fn derive_from_source(fs: &XmpFS, path: &OsString, source: &OsString)
{
    let mut table = TABLE!(fs.table);

    if !table.contains_key(path, source)
    {
        table.derive(path, source).expect("deriving");
        table.flush().expect("flushing");
    }
}

pub fn derive_from_toml(fs: &XmpFS, toml: toml::Value, cmdline: String)
{
    use regex::Regex;
    let get_regex_field = |field: &str| -> Option<&str> {
        if let Some(f) = toml.get(field)
        {
            let f = f.as_str().unwrap();
            let regex = Regex::new(f).unwrap();
            let cap = regex.captures(cmdline.as_str()).expect("did not match");
            cap.get(1).map(|c| c.as_str())
        }
        else
        {
            None
        }
    };
    let input = get_regex_field("input");
    let output = get_regex_field("output");

    if let (Some(output), Some(input)) = (output, input)
    {
        let input_path = format!("{}/{}", *BASE_PATH, input);
        let output_path = format!("{}/{}", *BASE_PATH, output);

        let mut table = TABLE!(fs.table);

        if !table.contains_key(&output_path, &input_path)
        {
            table.derive(&output_path, &input_path).expect("creating new");
            table.flush().expect("flushing");
        }
    }
}

pub fn check_for_known_program(fs: &XmpFS, req: &fuser::Request) -> Option<toml::Value>
{
    match get_cmdline_output(req).as_ref().map(|v| v.as_slice())
    {
        Some([program_name, ..]) =>
        {
            let s = program_name_from_path(program_name);
            if fs.known_programs.contains(&s)
            {
                let path = format!("{}/exe/{}", *BASE_PATH, s.to_str().unwrap());
                let path = Path::new(&path);

                assert!(path.exists());

                let toml = std::fs::read_to_string(path).expect("could not find attested program");
                toml.parse::<toml::Value>().ok()
            }
            else
            {
                None
            }
        },
        _ => None,
    }
}

pub fn get_parent_process(pid: u32) -> Option<u32>
{
    let path = format!("/proc/{}/status", pid);
    let path = Path::new(&path);
    let file = File::open(path).unwrap();

    let reader = BufReader::new(file);
    for line in reader.lines()
    {
        if let Ok(line) = line
        {
            if line.starts_with("PPid:")
            {
                let mut iter = line.split_ascii_whitespace();
                return iter.nth(1).map(|s| s.parse().unwrap());
            }
        }
    }
    None
}

pub fn set_lattice_of_new_file(path: &OsStr, program: &Program) -> Result<(), i32>
{
    let mut pathbuf = PathBuf::new();
    pathbuf.push(&*BASE_PATH);
    pathbuf.push(path);
    let path = pathbuf.as_path();

    set_output_label(path, program.integrity.clone());
    Ok(())
}
