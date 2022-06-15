use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Write},
    path::Path,
    rc::Rc,
};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use toml::Value;

use crate::policy::*;

#[allow(dead_code)]
type Node = Rc<RefCell<TableEntry>>;

use std::cell::RefCell;

const SAVE_PATH: &str = "/tmp/dropbox_folder/.table";


#[derive(Debug, Serialize, Deserialize)]
pub struct TableEntry
{
    labels:   Vec<String>,
    name:     String,
    children: HashMap<String, Rc<RefCell<TableEntry>>>,
    parent:   Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct Table
{
    //map:   HashMap<String, Rc<RefCell<TableEntry>>>,
    //#[serde(skip)]
    table: HashMap<String, Rc<RefCell<TableEntry>>>,
}

unsafe impl Send for Table {}

impl TableEntry
{
    fn from_file(path: impl AsRef<Path>, parent: Option<Vec<String>>) -> std::io::Result<Self>
    {
        // Multiple?
        let path = path.as_ref();
        let toml = read_tag(path, "user.label")?.parse::<Value>()?;
        let labels = &toml["labels"];
        /*if !labels.is_array()
        {
            return Err(Error::new(ErrorKind::InvalidInput, "un-recognizedformat, not table"));
        }*/


        /*let labels = labels.as_array().unwrap();
        let mut vec = Vec::with_capacity(labels.len());

        for label in labels
        {
            if !label.is_table()
            {
                return Err(Error::new(ErrorKind::InvalidInput, "un-recognizedformat, not object"));
            }
            let map = label.as_table().unwrap();

            let name = map["name"].as_str().expect("getting string").to_owned();
            let value = map["value"].to_string();
            vec.push(format!("{} {}", name, value));
        }*/

        if !labels.is_table()
        {
            return Err(Error::new(ErrorKind::InvalidInput, "un-recognizedformat, not object"));
        }
        let map = labels.as_table().unwrap();

        let label_name = map["name"].as_str().expect("getting string").to_owned();
        let label_value = map["value"].to_string();

        let name = path.file_name().unwrap().to_os_string().to_str().unwrap().to_string();
        Ok(TableEntry {
            labels: vec![format!("{} {}", label_name, label_value)],
            name,
            children: HashMap::new(),
            parent,
        })
    }
}

/*
pub struct Table
{
    map:   HashMap<String, Rc<RefCell<TableEntry>>>,
    table: HashMap<String, Rc<RefCell<TableEntry>>>,
}
*/

impl Table
{
    fn _get_name(path: impl AsRef<Path>) -> String
    {
        path.as_ref()
            .file_name()
            .expect("Getting filename")
            .to_str()
            .expect("to_str")
            .to_owned()
    }

    fn get_name(path: impl AsRef<Path>) -> String
    {
        path.as_ref().as_os_str().to_str().expect("to_str").to_owned()
    }

    fn _delete(&mut self, name: String) -> std::io::Result<()>
    {
        if let Some(set) = self.table.remove(&name)
        {
            let set = set.borrow();
            if let Some(parent) = &set.parent
            {
                // @TODO, handle this
                let mut parent =
                    self.table.get(parent.first().unwrap()).expect("getting parent").borrow_mut();
                parent.children.remove(&name);

                if parent.children.is_empty()
                {
                    let name = parent.name.clone();
                    drop(parent);
                    return self._delete(name);
                }
            }
            Ok(())
        }
        else
        {
            Err(Error::new(ErrorKind::NotFound, format!("Did not find {}", name)))
        }
    }

    // Unfortunanty, map_iter cannot be implemented with recursive_map_iter,
    // because the `table.insert(...)` line would require the closure to be
    // FnMut :))
    fn map_iter(
        map: HashMap<String, Rc<RefCell<TableEntry>>>,
    ) -> HashMap<String, Rc<RefCell<TableEntry>>>
    {
        let mut table = HashMap::new();
        for (name, rc) in map.iter()
        {
            Self::_map_iter((name, rc), &mut table);
        }
        table
    }

    fn _map_iter<'a>(
        (name, rc): (&str, &Rc<RefCell<TableEntry>>),
        table: &mut HashMap<String, Rc<RefCell<TableEntry>>>,
    )
    {
        table.insert(name.to_owned(), Rc::clone(rc));
        for (name, rc) in rc.borrow().children.iter()
        {
            Self::_map_iter((name, rc), table);
        }
    }

    fn recursive_map_iter<F>(map: &HashMap<String, Node>, f: F)
    where
        F: Fn(&String, &Node),
    {
        for elem in map.iter()
        {
            Self::_recursive_map_iter(elem, &f);
        }
    }

    fn _recursive_map_iter<F>((name, node): (&String, &Node), f: &F)
    where
        F: Fn(&String, &Node),
    {
        f(name, node);
        for elem in node.borrow().children.iter()
        {
            Self::_recursive_map_iter(elem, f);
        }
    }
}


/* public methods here ~! */
impl Table
{
    pub fn from_file() -> std::io::Result<Self>
    {
        let file = std::fs::File::open(SAVE_PATH)?;

        let reader = std::io::BufReader::new(file);
        let map: HashMap<String, Node> = serde_json::from_reader(reader)?;

        Ok(Table {
            table: Self::map_iter(map)
        })
    }

    pub fn contains<P: AsRef<Path>>(&self, dataset: P) -> bool
    {
        self.table.contains_key(&Self::get_name(&dataset))
    }

    pub fn flush(&self) -> std::io::Result<()>
    {
        let mut file = std::fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(SAVE_PATH)?;

        let top_level_map: HashMap<_, _> =
            self.table.iter().filter(|(_, rc)| rc.borrow().parent.is_none()).collect();

        let content = serde_json::to_string(&top_level_map)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn delete(&mut self, _dataset: impl AsRef<Path>) -> std::io::Result<()>
    {
        //let name = Self::get_name(dataset);
        //self._delete(name)
        Ok(())
    }

    pub fn insert<P: AsRef<Path>>(&mut self, new: P) -> std::io::Result<()>
    {
        let name = Self::get_name(&new);
        self.table
            .insert(name, Rc::new(RefCell::new(TableEntry::from_file(&new, None)?)));
        Ok(())
    }

    // @TODO: Use absolute path OR inode
    pub fn derive<P: AsRef<Path>>(&mut self, new: P, from: P) -> std::io::Result<()>
    {
        let new_name = Self::get_name(&new);
        let from_name = Self::get_name(&from);

        let entry = self
            .table
            .entry(from_name.clone())
            .or_insert(Rc::new(RefCell::new(TableEntry::from_file(&from, None)?)));

        let new_rc = Rc::new(RefCell::new(TableEntry::from_file(&new, Some(vec![from_name]))?));
        entry.borrow_mut().children.insert(new_name.clone(), Rc::clone(&new_rc));
        self.table.insert(new_name, new_rc);


        Ok(())
    }

    pub fn contains_key<P: AsRef<Path>>(&self, new: P, from: P) -> bool
    {
        self.table
            .get(&Self::get_name(&from))
            .map(|e| e.borrow().children.contains_key(&Self::get_name(&new)))
            .unwrap_or(false)
    }

    pub fn rename<P: AsRef<Path>>(&mut self, new: P, old: P) -> std::io::Result<()>
    {
        let new_name = Self::get_name(&new);
        let old_name = Self::get_name(&old);

        if let Some(entry) = self.table.remove(&old_name)
        {
            entry.borrow_mut().name = new_name.clone();

            // Move the parent's RC into the new hashmap bucket with the new name
            if let Some(parent) = &entry.borrow().parent
            {
                let parent = self.table.get(parent.first().unwrap()).unwrap();
                let parent_rc = parent.borrow_mut().children.remove(&old_name).unwrap();
                parent.borrow_mut().children.insert(new_name.clone(), parent_rc);
            }

            self.table.insert(new_name, entry);
        }
        else
        {
            return Err(Error::new(ErrorKind::NotFound, format!("Did not find {}", old_name)));
        }


        Ok(())
    }

    pub fn revoke<P: AsRef<Path>>(&mut self, dataset: P) -> std::io::Result<()>
    {
        let dataset_name = Self::get_name(&dataset);

        if !self.table.contains_key(&dataset_name)
        {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Could not find dataset {:?}", dataset_name),
            ));
        }


        let rc = self.table.remove(&dataset_name).unwrap();

        let revoke_func = |name: &String, _: &Node| {
            std::fs::remove_file(name).expect("removing file");
        };

        revoke_func(&dataset_name, &rc);
        Self::recursive_map_iter(&rc.borrow().children, revoke_func);

        rc.borrow_mut().children.clear();

        if let Some(ref parent) = rc.borrow().parent
        {
            self.table
                .get_mut(parent.first().unwrap())
                .unwrap()
                .borrow_mut()
                .children
                .remove(&dataset_name);
        }
        self.table
            .retain(|_, v| !(Rc::strong_count(v) < 1 && v.borrow().parent.is_some()));

        Ok(())
    }
}

fn _format(
    level: usize,
    mut builder: String,
    entry: &Rc<RefCell<TableEntry>>,
    is_last: bool,
) -> String
{
    let entry = entry.borrow();

    let prefix = if level > 1 { "├──" } else { "└──" };
    let prefix = if is_last { "└──" } else { prefix };


    //let labels = entry.labels.join(", ");
    let labels = entry
        .labels
        .iter()
        .map(|s| {
            let (_label, num) = s.split_once(' ').unwrap();
            let num: i64 = num.parse().unwrap();
            lattice_color(num)
        })
        .collect::<Vec<_>>()
        .join(",");



    let my_text = &format!(
        "{} {}label {}",
        Table::_get_name(&entry.name).green(),
        "@".truecolor(253, 141, 28),
        labels
    );


    let l = if level == 1 { 0 } else { level - 1 };
    let pretext = format!("{:level$}{}", "", prefix, level = l * 4);



    builder.push_str(&format!("{} {}\n", pretext, my_text));

    let len = entry.children.len();
    for (i, (_key, val)) in entry.children.iter().enumerate()
    {
        let is_last = i == len - 1;
        builder = _format(level + 1, builder, val, is_last);
    }
    builder
}

impl std::fmt::Display for Table
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let mut builder = String::new();
        builder.push_str("DATASETS:\n");

        let root_level_map: Vec<_> =
            self.table.values().filter(|e| e.borrow().parent.is_none()).collect();

        let len = root_level_map.len();
        for (i, val) in root_level_map.into_iter().enumerate()
        {
            let is_final = i == len - 1;
            builder = _format(1, builder, val, is_final);
        }
        write!(f, "{}", builder)
    }
}

fn lattice_color(linear_value: i64) -> String
{
    match linear_value
    {
        1 => format!("1 ({})", "private".red()),
        2 => format!("2 ({})", "sensitive".yellow()),
        3 => format!("3 ({})", "public".bright_green()),
        _ => unreachable!(),
    }
}
