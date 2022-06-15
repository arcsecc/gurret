mod handler;
pub use handler::MockHandler;

mod checker;
pub use checker::{get_metadata_checker, RustChecker};

mod dynamic;
use std::path::Path;

pub use dynamic::DynamicMetadata;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum Operation
{
    Write,
    Read,
    Open,
    Create,
    // etc.
}

impl std::fmt::Display for Operation
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        write!(f, "{:?}", self)
    }
}


pub trait Metadata
{
    type Item;
    fn check(&self, op: Operation) -> bool;
    fn update(&self) -> std::io::Result<()>;
    fn access(&self) -> std::io::Result<Self::Item>;
}


//
//type RetType<'a> = Vec<Box<dyn Metadata>;
type RetType<'a> = Vec<DynamicMetadata>;
//type RetType = Vec<()>;
//type RetType<'a> = FilterMap<slice::Iter<'a, RetIn>, &'static fn(&DirEntry)
// -> RetT>;


pub trait MetadataHandler
{
    /*
     * Return the metadata changes (if any) given an operation and a file.
     * The handler should run the `check` function for each field in addition
     * to some standard metadatas, like access(time).
     */
    //fn changes(&self, file: &Path, operation: Operation) -> Option<Vec<Box<dyn
    // Metadata>>>;
    fn changes(&mut self, file: &Path, operation: Operation) -> Vec<DynamicMetadata>;


    /*
     * Send the batched up changes to the remote. For the thesis, it is
     * sufficient to just send the file to an echo server.
     */
    fn update_remote(&self);
}
