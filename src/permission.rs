// Later this should be picked up from a config file or something
const USER_PERMISSION_PATH: &str = "/tmp/dropbox_folder/.user-clearance";
const PROGRAM_CLEARANCE_PATH: &str = "/tmp/dropbox_folder/exe";
//use std::io::Write;

#[allow(dead_code)]
const PERMISSION_DENIED: i32 = 13;
#[allow(dead_code)]
const IO_ERROR: i32 = 5;
#[allow(dead_code)]
const NO_SUCH_FILE_OR_DIRECTORY: i32 = 2;

#[allow(dead_code)]
const MAX_FILE_NAME_LENGTH: usize = 255;

use std::{ffi::OsString, path::Path};

//use serde_derive::Deserialize;
use toml::Value;

use crate::{lattice::*, policy::*, XmpFS};

fn get_file_lvalue(path: &str, ltype: &LatticeType) -> Result<LatticeValue, i32>
{
    let tag = match read_tag(path, "user.label").map(|s| s.parse::<Value>())
    {
        Ok(Ok(tag)) => tag,
        e =>
        {
            println!("no label {:?}", e);
            return Ok(ltype.default());
        },
    };


    if let Some(Value::Array(array)) = tag.get("labels")
    {
        // just pick first for now
        let table = &array[0];
        let name = table.get("name").unwrap().as_str().unwrap();
        match name
        {
            "linear" => Ok(LatticeValue::Number(table.get("value").unwrap().as_integer().unwrap())),
            _ => panic!("un-recognized lattice type"),
        }
    }
    else
    {
        // No lattice present, just assume linear lattice with val = 3
        Ok(LatticeValue::Number(3))
    }
}


#[allow(dead_code)]
fn get_lattice() -> (LatticeType, impl Lattice, LatticeValue)
{
    let program = std::fs::read_to_string(format!("{}", USER_PERMISSION_PATH)).unwrap();
    let program = program.parse::<Value>().unwrap();
    if let Some(Value::Table(table)) = program.get("clearance")
    {
        let name = table.get("name").unwrap().as_str().unwrap();
        match name
        {
            "linear" =>
            {
                let ltype = LatticeType::LinearNumber;
                let lattice = create_lattice(&ltype);
                let lvalue =
                    LatticeValue::Number(table.get("value").unwrap().as_integer().unwrap());

                (ltype, lattice, lvalue)
            },
            _ => panic!("un-recognized lattice type"),
        }
    }
    else
    {
        panic!("un-recognized format")
    }
}

/*macro_rules! file_path {
    ($($arg:tt)*) => {
        {
        let mut file_name: [u8; MAX_FILE_NAME_LENGTH] = [0; MAX_FILE_NAME_LENGTH];
        let mut file_ref: &mut [u8] = &mut file_name;

        write!(file_ref, $($arg)*);
        let p = std::str::from_utf8(&file_name).unwrap();
        Path::new(&p)
        }
    };
}*/

// A little scuffed, but here is what is does,
// Either it returns the gate label, or the confidentiality label
pub fn get_toml_label(toml: &toml::Value) -> LabelResponse
{
    let get_val = |s: &str, toml: &toml::Value| {
        toml.get(s).map(|val| {
            let table = val.as_table().unwrap();
            let r#type = match table.get("name").unwrap().as_str().unwrap()
            {
                "linear" => LatticeType::LinearNumber,
                _ => unreachable!(),
            };
            let val = table.get("value").unwrap().to_string();
            let val = LatticeValue::from_string(&r#type, &val);

            (r#type, val)
        })
    };

    //get_val("gate").unwrap_or_else()
    if let Some(toml) = toml.get("gate")
    {
        if let (Some(confidentiality), Some(integrity)) =
            (get_val("confidentiality", toml), get_val("integrity", toml))
        {
            LabelResponse::Gate {
                integrity,
                confidentiality,
            }
        }
        else
        {
            panic!("Should not reach here~!");
        }
    }
    else
    {
        let pair = get_val("confidentiality", toml).unwrap();
        LabelResponse::Confidentiality(pair)
    }
}


pub fn set_output_label(new_dataset: impl AsRef<Path>, (r#type, val): LatticePair)
{
    let new_dataset_name = new_dataset.as_ref().file_name().unwrap();

    let lattices = format!("{{name=\"{}\",value={}}}", r#type, val);
    let content = format!("labels = {}", lattices);

    crate::policy::set_policy(&new_dataset, content)
        .expect(&format!("setting policy {:?}", new_dataset_name));
}


pub enum RequestType
{
    READ,
    //WRITE,
}


pub enum LabelResponse
{
    Confidentiality(LatticePair),
    Gate
    {
        integrity:       LatticePair,
        confidentiality: LatticePair,
    },
}

impl XmpFS
{
    pub fn check_permission(
        &self,
        req: &fuser::Request,
        ino: u64,
        _request_type: RequestType,
    ) -> Result<i32, i32>
    {
        let program = self.programs.get(&req.pid()).unwrap();

        let program_label = &program.confidentiality;
        let lattice_type = &program_label.0;

        let lattice = create_lattice(lattice_type);

        let full_path = self.inode_to_path.get(&ino).unwrap();
        let file_label = get_file_lvalue(full_path.to_str().unwrap(), lattice_type)?;


        match lattice.compare(&program_label.1, &file_label)
        {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Ok(0),
            std::cmp::Ordering::Greater => Err(PERMISSION_DENIED),
        }
    }

    pub fn get_confidentiality_label(&self, program: OsString) -> Option<LabelResponse>
    {
        if self.known_programs.contains(&program)
        {
            let toml = std::fs::read_to_string(format!(
                "{}/{}",
                PROGRAM_CLEARANCE_PATH,
                program.as_os_str().to_str().unwrap()
            ));
            let toml = toml.unwrap().parse::<toml::Value>().unwrap();
            Some(get_toml_label(&toml))
        }
        else
        {
            self._get_file_label(program)
                .or(Some(LabelResponse::Confidentiality(lattice_pair_default())))
        }
    }

    pub fn get_file_label(&self, path: impl AsRef<Path>) -> Option<LabelResponse>
    {
        let path = path.as_ref().file_stem().unwrap().to_os_string();
        println!("{:?}", path);
        self.get_confidentiality_label(path)
    }

    fn _get_file_label(&self, path: impl AsRef<Path>) -> Option<LabelResponse>
    {
        let tag = match read_tag(path, "user.label").map(|s| s.parse::<Value>())
        {
            Ok(Ok(tag)) => tag,
            _ =>
            {
                return Some(LabelResponse::Confidentiality(lattice_pair_default()));
            },
        };


        if let Some(Value::Array(array)) = tag.get("labels")
        {
            // just pick first for now
            let table = &array[0];
            let name = table.get("name").unwrap().as_str().unwrap();
            match name
            {
                "linear" =>
                {
                    let number = table.get("value").unwrap().as_integer().unwrap();
                    let pair = (LatticeType::LinearNumber, LatticeValue::Number(number));
                    let resp = LabelResponse::Confidentiality(pair);
                    Some(resp)
                },
                _ => panic!("un-recognized lattice type"),
            }
        }
        else
        {
            // No lattice present, just assume linear lattice with val = 3
            return Some(LabelResponse::Confidentiality(lattice_pair_default()));
        }
    }
}
