use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::metadata::{
    get_metadata_checker, DynamicMetadata, Metadata, MetadataHandler, Operation,
};


pub struct MockHandler
{
    root_folder: PathBuf,
    check_cache: HashMap<(PathBuf, Operation), Box<dyn Metadata<Item = ()>>>,

    exec_cache: HashMap<(PathBuf, Operation), DynamicMetadata>,

    directory_cache: HashMap<PathBuf, Vec<std::fs::DirEntry>>,
}

impl MockHandler
{
    pub fn new() -> Self
    {
        let mut root_folder = PathBuf::new();
        // @TODO: SHould be in config
        root_folder.push("/tmp/dropbox_folder/metadata/");

        Self {
            root_folder,
            check_cache: HashMap::new(),
            exec_cache: HashMap::new(),

            directory_cache: HashMap::new(),
        }
    }
}

/*use lazy_static::lazy_static;
lazy_static! {
    static mut HASHMAP: HashMap<PathBuf, DynamicMetadata> = HashMap::new();
}*/
static mut HASHMAP: Option<HashMap<PathBuf, DynamicMetadata>> = None; //HashMap::new();

pub fn run<'m>(mock: &'m mut MockHandler, file: &Path, operation: Operation)
    -> Vec<DynamicMetadata>
{
    let file_stem = file.file_stem().unwrap();
    let mut root = mock.root_folder.clone();
    root.push(file_stem);


    std::fs::read_dir(root.clone())
        .unwrap()
        .flatten()
        .map(|entry| {
            let mut check = entry.path();
            check.push("check");

            let checker = mock
                .check_cache
                .entry((check.clone(), operation))
                .or_insert_with(|| get_metadata_checker(&check));

            //let checker = get_metadata_checker(&check);
            let ans = checker.check(operation);

            //ans.then(|| {
            let mut exec = entry.path();
            exec.push("execute");


            /*let map = unsafe
            {
                if HASHMAP.is_none()
                {
                    HASHMAP = Some(HashMap::new());
                }
                HASHMAP.as_mut().unwrap()
            };

            if !map.contains_key(&exec)
            {
                map.insert(exec.clone(), DynamicMetadata::new(exec.clone()));
            }
            map.get(&exec).unwrap()*/

            DynamicMetadata::new(exec.clone())
        })
        .collect()
}


impl MetadataHandler for MockHandler
{
    fn changes(&mut self, file: &Path, operation: Operation) -> Vec<DynamicMetadata>
    {
        let res = run(self, file, operation);

        res
        //cache_everything(self, file, operation)
    }

    fn update_remote(&self)
    {
        todo!()
    }
}
