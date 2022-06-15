use std::{ffi::OsStr, path::Path};


/*const POLICY_STR: &str = "user.policy";

pub trait Policy
{
    //fn execute(src: &str);
    //fn check_policy(file: S) -> bool; // Mabye return an object here that has an
    // .can_access() or something
    fn set_policy(file: impl AsRef<Path>, value: impl AsRef<[u8]>) -> Result<(), std::io::Error>
    {
        tag_file(file, POLICY_STR, value)
    }

    fn has_access(file: impl AsRef<Path>) -> std::io::Result<bool>;
}*/

pub fn tag_file(
    file: impl AsRef<Path>,
    name: impl AsRef<OsStr>,
    value: impl AsRef<[u8]>,
) -> Result<(), std::io::Error>
{
    xattr::set(file, name, value.as_ref())?;
    Ok(())
}

pub fn set_policy(file: impl AsRef<Path>, labels: String) -> std::io::Result<()>
{
    tag_file(file, "user.label", labels)
}

#[allow(dead_code)]
pub fn read_tag(
    file: impl AsRef<Path>,
    tag_name: impl AsRef<OsStr>,
) -> Result<String, std::io::Error>
{
    let p = xattr::get(file, tag_name)?;
    match p
    {
        Some(xattr) => Ok(String::from_utf8(xattr).unwrap()),
        _ => Ok("".to_string()),
    }
}


/*pub struct Dummy;
impl Policy for Dummy
{
    // When implementing this for real, check if the connection is open
    fn has_access(file: impl AsRef<Path>) -> Result<bool, std::io::Error>
    {
        let s = read_tag(file, POLICY_STR)?;
        Ok(s == "true")
    }
}
*/
// https://docs.rs/meval/0.2.0/meval/
