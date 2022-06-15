use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    io::ErrorKind,
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    lattice::{Lattice, LatticePair, *},
    metadata::*,
    permission::{self, *},
    table::*,
    BASE_PATH, TABLE,
};

const BLOCK_SIZE: u32 = 512;

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};
use libc::{
    c_int, EINVAL, EIO, ENOENT, ENOSYS, EPERM, O_ACCMODE, O_APPEND, O_CREAT, O_EXCL, O_RDONLY,
    O_RDWR, O_TRUNC, O_WRONLY,
};
use log::error;

use crate::config::*;

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct DirInfo
{
    ino:  u64,
    name: OsString,
    kind: FileType,
}

#[derive(Debug, Clone)]
pub struct Program
{
    pub program_name:    OsString,
    pub resources:       HashSet<OsString>,
    pub integrity:       LatticePair,
    pub confidentiality: LatticePair,
    pub gate:            bool,
}


use std::path::PathBuf;
type MetadataId = u64;

#[derive(Debug, Clone)]
pub enum MSource
{
    Value(OsString),
    Parent
    {
        parent_ids: MetadataId,
        own:        OsString,
    },
}
use MSource::*;

#[derive(Default, Debug)]
pub struct Broker
{
    pub available_id:    MetadataId,
    pub id_to_metadata:  Vec<MSource>,
    pub file_name_to_id: HashMap<OsString, MetadataId>,
}

impl Broker
{
    fn get_name(&self, id: MetadataId) -> &OsString
    {
        match self.id_to_metadata.get(id as usize)
        {
            Some(source) => match source
            {
                MSource::Value(ref p) => p,
                MSource::Parent {
                    parent_ids,
                    own,
                } => &own,
            },
            _ => unreachable!(),
        }
    }

    fn map_name_to_id(&mut self, program_name: OsString, parent: Option<OsString>) -> MSource
    {
        let id = self.available_id;
        self.available_id += 1;

        self.file_name_to_id.insert(program_name.clone(), id);

        let source = match parent
        {
            Some(parent) =>
            {
                let id = self.file_name_to_id.get(&parent).unwrap();
                MSource::Parent {
                    parent_ids: *id, own: program_name
                }
            },
            _ => MSource::Value(program_name),
        };

        self.id_to_metadata.push(source.clone());
        source
    }

    fn get_program(&mut self, program_name: OsString, parent: Option<OsString>) -> PProgram
    {
        PProgram {
            m_source: self.map_name_to_id(program_name, parent)
        }
    }
}


#[derive(Debug)]
pub struct PProgram
{
    pub m_source: MSource,
}


impl Program
{
    fn new(program_name: OsString, label: LabelResponse) -> Self
    {
        match label
        {
            // If we read something with higher lattice, we get tainted
            LabelResponse::Confidentiality(label) => Self {
                program_name,
                resources: HashSet::new(),
                integrity: lattice_pair_default(),
                confidentiality: label,
                gate: false,
            },
            // If it's a gate, we get special priveleges
            LabelResponse::Gate {
                integrity,
                confidentiality,
            } => Self {
                program_name,
                resources: HashSet::new(),
                integrity,
                confidentiality,
                gate: true,
            },
        }
    }

    fn open(&mut self, file: OsString, resp: Option<LabelResponse>)
    {
        // Gates keep their current privelege level
        if !self.gate
        {
            if let Some(resp) = resp
            {
                match resp
                {
                    // If we read something with higher lattice, we get tainted
                    LabelResponse::Confidentiality(labels) =>
                    {
                        let my_label = &self.integrity;
                        let recv_label = &labels;
                        let lattice = crate::lattice::create_lattice(&my_label.0);

                        if lattice.compare(&my_label.1, &recv_label.1)
                            == std::cmp::Ordering::Greater
                        {
                            self.integrity = labels;
                        }
                    },
                    // If it's a gate, we get special priveleges
                    LabelResponse::Gate {
                        integrity,
                        confidentiality,
                    } =>
                    {
                        self.integrity = integrity;
                        self.confidentiality = confidentiality;
                    },
                };
            }
        }

        if file != self.program_name
        {
            self.resources.insert(file);
        }
    }
}


unsafe impl Send for XmpFS {}
pub struct XmpFS
{
    pub counter: u64,

    pub inode_to_path: HashMap<u64, OsString>,
    pub path_to_inode: HashMap<OsString, u64>,

    pub opened_directories: HashMap<u64, Vec<DirInfo>>,
    pub opened_files:       HashMap<u64, std::fs::File>,
    pub table:              Arc<Mutex<Table>>,
    pub known_programs:     Vec<OsString>,

    pub programs: HashMap<u32, Program>,

    pub pprograms: HashMap<OsString, PProgram>,

    pub han: Broker,

    metadata_handler: Box<dyn MetadataHandler>,
}


impl XmpFS
{
    pub fn new() -> XmpFS
    {
        let mut map: HashMap<OsString, PProgram> = HashMap::new();
        let mut han = Broker::default();




        let name: OsString = "/tmp/dropbox_folder/file0".into();
        let pp = han.get_program(name.clone(), None);
        map.insert(name.clone(), pp);
        std::fs::File::create(name);




        for i in 1..100
        {
            let name: OsString = format!("/tmp/dropbox_folder/file{i}").into();
            let parent: OsString = format!("/tmp/dropbox_folder/file{}", i - 1).into();
            let pp = han.get_program(name.clone(), Some(parent));
            std::fs::File::create(&name);

            map.insert(name, pp);
        }

        let table = Arc::new(Mutex::new(Table::from_file().unwrap_or_else(|_| Table::default())));
        XmpFS {
            han,
            counter: 1,
            inode_to_path: HashMap::with_capacity(1024),
            path_to_inode: HashMap::with_capacity(1024),
            opened_directories: HashMap::with_capacity(2),
            opened_files: HashMap::with_capacity(2),
            table,
            known_programs: Vec::new(),
            programs: HashMap::new(),
            pprograms: map,
            /*derive:             None,
             *dependency_map:     HashMap::new(), */
            metadata_handler: Box::new(MockHandler::new()),
        }
    }

    pub fn get_known_programs(&self) -> Vec<OsString>
    {
        std::fs::read_dir(format!("{}/exe", *BASE_PATH))
            .expect("reading dir exe")
            .into_iter()
            .flatten()
            .map(|buf| buf.path().file_name().unwrap().to_os_string())
            .collect()
    }

    #[allow(dead_code)]
    pub fn log_operation(&mut self, ino: u64, req: &Request, prefix: &str)
    {
        let cmdline = std::fs::read_to_string(format!("/proc/{}/cmdline", req.pid())).unwrap();
        let (program, _) = get_name_and_file(&cmdline);
        let full_path = self.inode_to_path.get(&ino).unwrap();
        if full_path.to_str().unwrap().contains(".attested-programs")
        {
            return;
        }

        let temp_name = get_temp_name(full_path);
        let path = Path::new(self.inode_to_path.get(&ino).unwrap());
        let parent = path.parent().unwrap();

        let entry_path = parent.join(temp_name);

        if !file_for_temp(entry_path.as_os_str().to_str().unwrap())
        {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(entry_path)
                .expect("opening file");

            use std::io::prelude::*;
            let content = format!("{} {} {}\n", prefix, program, log_time());

            file.write_all(content.as_bytes()).unwrap();
        }
    }

    pub fn populate_root_dir(&mut self)
    {
        let rootino = self.add_inode(OsStr::from_bytes(BASE_PATH.as_bytes()));
        self.known_programs = self.get_known_programs();
        assert_eq!(rootino, 1);
    }

    pub fn add_inode(&mut self, path: &OsStr) -> u64
    {
        let ino = self.counter;
        self.counter += 1;
        self.path_to_inode.insert(path.to_os_string(), ino);
        self.inode_to_path.insert(ino, path.to_os_string());
        ino
    }

    pub fn add_or_create_inode(&mut self, path: impl AsRef<Path>) -> u64
    {
        let path = path.as_ref().as_os_str().to_os_string();

        if let Some(x) = self.path_to_inode.get(&path)
        {
            return *x;
        }

        self.add_inode(&path)
    }

    pub fn get_inode(&self, path: impl AsRef<Path>) -> Option<u64>
    {
        self.path_to_inode.get(path.as_ref().as_os_str()).map(|x| *x)
    }

    pub fn unregister_ino(&mut self, ino: u64)
    {
        if !self.inode_to_path.contains_key(&ino)
        {
            return;
        }
        self.path_to_inode.remove(&self.inode_to_path[&ino]);
        self.inode_to_path.remove(&ino);
    }

    pub fn derive_data(&self, _req: &Request, entry_path: &OsString, program: &Program)
    {
        for file in program.resources.iter()
        {
            derive_from_source(self, entry_path, file);
        }
    }
}

fn ft2ft(t: std::fs::FileType) -> FileType
{
    match t
    {
        x if x.is_symlink() => FileType::Symlink,
        x if x.is_dir() => FileType::Directory,
        x if x.is_file() => FileType::RegularFile,
        x if x.is_fifo() => FileType::NamedPipe,
        x if x.is_char_device() => FileType::CharDevice,
        x if x.is_block_device() => FileType::BlockDevice,
        x if x.is_socket() => FileType::Socket,
        _ => FileType::RegularFile,
    }
}

fn meta2attr(m: &std::fs::Metadata, ino: u64) -> FileAttr
{
    FileAttr {
        ino,
        size: m.size(),
        blocks: m.blocks(),
        atime: m.accessed().unwrap_or(UNIX_EPOCH),
        mtime: m.modified().unwrap_or(UNIX_EPOCH),
        ctime: UNIX_EPOCH + Duration::from_secs(m.ctime().try_into().unwrap_or(0)),
        crtime: m.created().unwrap_or(UNIX_EPOCH),
        kind: ft2ft(m.file_type()),
        perm: m.permissions().mode() as u16,
        nlink: m.nlink() as u32,
        uid: m.uid(),
        gid: m.gid(),
        rdev: m.rdev() as u32,
        flags: 0,
        blksize: BLOCK_SIZE,
    }
}

fn errhandle(e: std::io::Error, not_found: impl FnOnce() -> ()) -> libc::c_int
{
    match e.kind()
    {
        ErrorKind::PermissionDenied => EPERM,
        ErrorKind::NotFound =>
        {
            not_found();
            ENOENT
        },
        e =>
        {
            error!("{:?}", e);
            EIO
        },
    }
}

impl Filesystem for XmpFS
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry)
    {
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(&name);

        let entry_inode = self.get_inode(&entry_path);

        match std::fs::symlink_metadata(entry_path)
        {
            Err(e) =>
            {
                reply.error(errhandle(e, || {
                    // if not found:
                    if let Some(ino) = entry_inode
                    {
                        self.unregister_ino(ino);
                    }
                }));
            },
            Ok(m) =>
            {
                let ino = match entry_inode
                {
                    Some(x) => x,
                    None =>
                    {
                        let parent_path = Path::new(&self.inode_to_path[&parent]);
                        let entry_path = parent_path.join(&name);
                        self.add_or_create_inode(entry_path)
                    },
                };

                let attr: FileAttr = meta2attr(&m, ino);

                reply.entry(&TTL, &attr, 1);
            },
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr)
    {
        //println!("getattr");
        if !self.inode_to_path.contains_key(&ino)
        {
            return reply.error(ENOENT);
        }


        let entry_path = Path::new(&self.inode_to_path[&ino]);

        match std::fs::symlink_metadata(entry_path)
        {
            Err(e) =>
            {
                reply.error(errhandle(e, || {
                    // if not found:
                    self.unregister_ino(ino);
                }));
            },
            Ok(m) =>
            {
                let attr: FileAttr = meta2attr(&m, ino);
                reply.attr(&TTL, &attr);
            },
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen)
    {
        if !self.inode_to_path.contains_key(&ino)
        {
            return reply.error(ENOENT);
        }

        let entry_path = Path::new(&self.inode_to_path[&ino]);
        /*if !self.programs.contains_key(&_req.pid())
        {
            // We do not track the current process, either it is an attested program opening
            // some
            // file, or a regular program opening an attested script

            let stem = entry_path.file_stem().unwrap().to_os_string();
            let resp = if self.known_programs.contains(&stem)
            {
                // Attested script
                self.get_confidentiality_label(stem).unwrap()
            }
            else
            {
                let pname = get_program_name(_req).unwrap();
                self.get_confidentiality_label(pname).unwrap()
            };

            self.programs
                .insert(_req.pid(), Program::new(get_program_name(_req).unwrap(), resp));
        }*/


        /*let changes = self.metadata_handler.changes(entry_path, Operation::Open);

        for change in changes
        {
            change.execute().expect("executing..");
        }*/

        /*use std::path::PathBuf;
        let mut path = PathBuf::new();
        //path.push("/tmp/dropbox_folder");

        path.push(entry_path);
        // println!("path: {}", path.display());



        //let t = std::time::Instant::now();

        while let Some(p) = self.pprograms.get(path.as_os_str())
        {
            self.metadata_handler.changes(&path, Operation::Open);
            if p.parent.is_none()
            {
                break;
            }
            //path.pop();
            path.clear();
            path.push(p.parent.as_ref().unwrap());
        }*/
        //let time = t.elapsed().as_micros();
        //


        //let t = std::time::Instant::now();
        let mut path = entry_path.to_path_buf();
        //println!("path {}", path.display());

        while let Some(p) = self.pprograms.get(path.as_os_str())
        {
            //println!("while path {}", path.display());
            match &p.m_source
            {
                MSource::Value(_) =>
                {
                    self.metadata_handler.changes(&path, Operation::Open);
                    break;
                },

                MSource::Parent {
                    own,
                    parent_ids,
                } =>
                {
                    path.clear();
                    path.push(own);
                    self.metadata_handler.changes(&path, Operation::Open);
                    let id = *parent_ids;

                    let parent_path = self.han.get_name(id);
                    path.push(parent_path);
                },
            }
        }


        /*
        fn traverse(operation: Operation, path: MetadataId, handler: &Broker, store: &mut Vec<i32>)
        {
            match handler.id_to_metadata
            {
                // Add the values in `changes(..)` to store (if any)
                Value(path) => store.extend(handler.changes(&path, operation)),
                Parent {
                    path,
                    parent_ids,
                } =>
                {
                    store.extend(handler.changes(&path, operation));
                    for id in parent_ids
                    {
                        traverse(operation, id, handler, store);
                    }
                },
            }
        }
        */


        /*if let Err(err) = self.check_permission(_req, ino, permission::RequestType::READ)
        {
            return reply.error(err);
        }*/


        let mut oo = std::fs::OpenOptions::new();


        let fl = flags as c_int;
        match fl & O_ACCMODE
        {
            O_RDONLY =>
            {
                /*println!("readonly");
                let resp = self.get_file_label(entry_path);
                self.programs
                    .get_mut(&_req.pid())
                    .unwrap()
                    .open(entry_path.as_os_str().to_os_string(), resp);*/

                oo.read(true);
                oo.write(false);
            },
            O_WRONLY =>
            {
                // println!("writeonly");
                oo.read(false);
                oo.write(true);
            },
            O_RDWR =>
            {
                // println!("readwrite");
                oo.read(true);
                oo.write(true);
            },
            _ => return reply.error(EINVAL),
        }

        oo.create(false);
        if fl & (O_EXCL | O_CREAT) != 0
        {
            error!("Wrong flags on open");
            return reply.error(EIO);
        }

        oo.append(fl & O_APPEND == O_APPEND);
        oo.truncate(fl & O_TRUNC == O_TRUNC);

        match oo.open(entry_path)
        {
            Err(e) => reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(f) =>
            {
                let fh = self.counter;
                self.counter += 1;

                /*if self.derive.is_none()
                {
                    self.derive = get_derived(&self, &_req);
                }*/
                self.opened_files.insert(fh, f);
                reply.opened(fh, 0);
            },
        }
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    )
    {
        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(name);

        let ino = self.add_or_create_inode(&entry_path);

        let mut oo = std::fs::OpenOptions::new();

        let fl = flags as c_int;
        match fl & O_ACCMODE
        {
            O_RDONLY =>
            {
                oo.read(true);
                oo.write(false);
            },
            O_WRONLY =>
            {
                oo.read(false);
                oo.write(true);
            },
            O_RDWR =>
            {
                oo.read(true);
                oo.write(true);
            },
            _ => return reply.error(EINVAL),
        }

        oo.create(fl & O_CREAT == O_CREAT);
        oo.create_new(fl & O_EXCL == O_EXCL);
        oo.append(fl & O_APPEND == O_APPEND);
        oo.truncate(fl & O_TRUNC == O_TRUNC);
        oo.mode(mode);

        match oo.open(&entry_path)
        {
            Err(e) => return reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(f) =>
            {
                let meta = match std::fs::symlink_metadata(entry_path)
                {
                    Err(e) =>
                    {
                        return reply.error(errhandle(e, || self.unregister_ino(ino)));
                    },
                    Ok(m) => meta2attr(&m, ino),
                };
                let fh = self.counter;
                self.counter += 1;

                //check_and_record_derive(self, _req);
                //self.log_operation(ino, _req, "create");

                /*if !self.pprograms.contains_key(&entry_path)
                {
                    let mut pp = PProgram::default();
                    pp.program_name = entry_path.clone();


                }*/


                // Process creating did not exist
                /*if !self.programs.contains_key(&_req.pid())
                {
                    let name = get_program_name(_req).unwrap();
                    let p = match get_parent_process(_req.pid())
                    {
                        // Some random program just created a new file
                        None =>
                        {
                            let resp = LabelResponse::Confidentiality(lattice_pair_strictest());
                            Program::new(name, resp)
                        },
                        Some(ppid) => match self.programs.get(&ppid)
                        {
                            Some(p) =>
                            {
                                let mut p = p.clone();
                                p.program_name = name;
                                p
                            },
                            _ =>
                            {
                                let resp = LabelResponse::Confidentiality(lattice_pair_strictest());
                                Program::new(name, resp)
                            },
                        },
                    };
                    self.programs.insert(_req.pid(), p);
                }
                let program = self.programs.get(&_req.pid()).unwrap();

                set_lattice_of_new_file(name, &program).expect("setting label");*/

                self.opened_files.insert(fh, f);
                reply.created(&TTL, &meta, 1, fh, 0);
            },
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    )
    {
        if !self.opened_files.contains_key(&fh)
        {
            return reply.error(EIO);
        }
        let size = size as usize;




        let mut b = Vec::with_capacity(size);
        b.resize(size, 0);

        use std::os::unix::fs::FileExt;
        let file = self.opened_files.get(&fh).unwrap();


        let file_size = file.metadata().unwrap().len();
        // Could underflow if file length is less than local_start
        let read_size = std::cmp::min(size, file_size.saturating_sub(offset as u64) as usize);

        let mut buffer = vec![0; read_size as usize];
        file.read_exact_at(&mut buffer, offset as u64).unwrap();
        reply.data(&buffer);

        /*let mut bo = 0;
        while bo < size
        {
            match f.read_at(&mut b[bo..], offset as u64)
            {
                Err(e) =>
                {
                    return reply.error(errhandle(e, || ()));
                },
                Ok(0) =>
                {
                    b.resize(bo, 0);
                    break;
                },
                Ok(ret) =>
                {
                    bo += ret;
                },
            };
        }

        reply.data(&b[..]);*/
    }

    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    )
    {
        if !self.opened_files.contains_key(&fh)
        {
            return reply.error(EIO);
        }

        let f = self.opened_files.get_mut(&fh).unwrap();

        use std::os::unix::fs::FileExt;
        let entry_path = self.inode_to_path.get(&_ino).unwrap();


        match f.write_all_at(data, offset as u64)
        {
            Err(e) => return reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                /*if let Some(program) = self.programs.get(&_req.pid())
                {
                    self.derive_data(_req, entry_path, program);
                }
                else
                {
                    if let Some(ppid) = get_parent_process(_req.pid())
                    {
                        let new_program =
                            self.programs.get(&ppid).expect("no parent found").clone();
                        self.derive_data(_req, entry_path, &new_program);
                        self.programs.insert(ppid, new_program);
                    }
                    else
                    {
                        panic!("No parent, no process");
                    }
                }*/

                reply.written(data.len() as u32);
            },
        };
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty)
    {
        if !self.opened_files.contains_key(&fh)
        {
            return reply.error(EIO);
        }

        let f = self.opened_files.get_mut(&fh).unwrap();

        match if datasync { f.sync_data() } else { f.sync_all() }
        {
            Err(e) => return reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                reply.ok();
            },
        }
    }

    fn fsyncdir(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty)
    {
        // I'm not sure how to do I with libstd
        reply.ok();
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    )
    {
        if !self.opened_files.contains_key(&fh)
        {
            return reply.error(EIO);
        }


        // remove dependencies
        /*let entry_path = Path::new(&self.inode_to_path[&_ino]);
        let name = entry_path.file_name().unwrap().to_os_string();
        let mut buf = std::path::PathBuf::new();
        buf.push(&*BASE_PATH);
        buf.push(name);*/

        //println!("release {:?}", buf.as_path());
        /*
        let path = buf.into_os_string();
        self.read_files.remove(&path);
        if let Some(program) = self.programs.get_mut(&_req.pid())
        {
            // Program have released every resource
            if program.release()
            {
                self.programs.remove(&_req.pid());
            }
        }
        */



        self.opened_files.remove(&fh);
        reply.ok();
    }

    fn opendir(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen)
    {
        //println!("opendir");
        if !self.inode_to_path.contains_key(&ino)
        {
            return reply.error(ENOENT);
        }

        let entry_path = Path::new(&self.inode_to_path[&ino]).to_owned();
        //println!("release {:?}", entry_path);

        match std::fs::read_dir(&entry_path)
        {
            Err(e) =>
            {
                reply.error(errhandle(e, || ()));
            },
            Ok(x) =>
            {
                let mut v: Vec<DirInfo> = Vec::with_capacity(x.size_hint().0);

                let parent_ino: u64 = if ino == 1
                {
                    1
                }
                else
                {
                    match entry_path.parent()
                    {
                        None => ino,
                        Some(x) => *self.path_to_inode.get(x.as_os_str()).unwrap_or(&ino),
                    }
                };

                v.push(DirInfo {
                    ino:  ino,
                    kind: FileType::Directory,
                    name: OsStr::from_bytes(b".").to_os_string(),
                });
                v.push(DirInfo {
                    ino:  parent_ino,
                    kind: FileType::Directory,
                    name: OsStr::from_bytes(b"..").to_os_string(),
                });

                for dee in x
                {
                    match dee
                    {
                        Err(e) =>
                        {
                            reply.error(errhandle(e, || ()));
                            return;
                        },
                        Ok(de) =>
                        {
                            let name = de.file_name().to_os_string();

                            let kind = de.file_type().map(ft2ft).unwrap_or(FileType::RegularFile);
                            let jp = entry_path.join(&name);
                            let ino = self.add_or_create_inode(jp);
                            v.push(DirInfo {
                                ino,
                                kind,
                                name,
                            });
                        },
                    }
                }
                let fh = self.counter;
                self.opened_directories.insert(fh, v);
                self.counter += 1;
                reply.opened(fh, 0);
            },
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    )
    {
        //println!("readdir");
        if !self.opened_directories.contains_key(&fh)
        {
            error!("no fh {} for readdir", fh);
            return reply.error(EIO);
        }

        let entries = &self.opened_directories[&fh];

        for (i, entry) in entries.iter().enumerate().skip(offset as usize)
        {
            if reply.add(entry.ino, (i + 1) as i64, entry.kind, &entry.name)
            {
                break;
            }
        }
        reply.ok();
    }

    fn releasedir(&mut self, _req: &Request, _ino: u64, fh: u64, _flags: i32, reply: ReplyEmpty)
    {
        //println!("releasedir");
        if !self.opened_directories.contains_key(&fh)
        {
            return reply.error(EIO);
        }

        self.opened_directories.remove(&fh);
        reply.ok();
    }

    fn readlink(&mut self, _req: &Request, ino: u64, reply: ReplyData)
    {
        //println!("readlink");
        if !self.inode_to_path.contains_key(&ino)
        {
            return reply.error(ENOENT);
        }

        let entry_path = Path::new(&self.inode_to_path[&ino]);

        match std::fs::read_link(entry_path)
        {
            Err(e) => reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(x) =>
            {
                reply.data(x.as_os_str().as_bytes());
            },
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    )
    {
        //println!("mkdir");
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(name);

        let ino = self.add_or_create_inode(&entry_path);
        match std::fs::create_dir(&entry_path)
        {
            Err(e) => reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                let attr = match std::fs::symlink_metadata(entry_path)
                {
                    Err(e) =>
                    {
                        return reply.error(errhandle(e, || self.unregister_ino(ino)));
                    },
                    Ok(m) => meta2attr(&m, ino),
                };

                reply.entry(&TTL, &attr, 1);
            },
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty)
    {
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(name);

        match std::fs::remove_file(&entry_path)
        {
            Err(e) => reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                let mut table = TABLE!(self.table);
                if table.delete(entry_path).is_ok()
                {
                    //table.flush().expect("flushing");
                }
                reply.ok();
            },
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty)
    {
        //println!("rmdir");
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(name);

        match std::fs::remove_dir(entry_path)
        {
            Err(e) => reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                reply.ok();
            },
        }
    }

    fn symlink(&mut self, _req: &Request, parent: u64, name: &OsStr, link: &Path, reply: ReplyEntry)
    {
        //println!("symlink");
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let entry_path = parent_path.join(name);
        let ino = self.add_or_create_inode(&entry_path);

        match std::os::unix::fs::symlink(&entry_path, link)
        {
            Err(e) => reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(()) =>
            {
                let attr = match std::fs::symlink_metadata(entry_path)
                {
                    Err(e) =>
                    {
                        return reply.error(errhandle(e, || self.unregister_ino(ino)));
                    },
                    Ok(m) => meta2attr(&m, ino),
                };

                reply.entry(&TTL, &attr, 1);
            },
        }
    }

    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    )
    {
        println!("rename");
        if !self.inode_to_path.contains_key(&parent)
        {
            return reply.error(ENOENT);
        }
        if !self.inode_to_path.contains_key(&newparent)
        {
            return reply.error(ENOENT);
        }

        let parent_path = Path::new(&self.inode_to_path[&parent]);
        let newparent_path = Path::new(&self.inode_to_path[&newparent]);
        let entry_path = parent_path.join(name);
        let newentry_path = newparent_path.join(newname);

        if entry_path == newentry_path
        {
            return reply.ok();
        }

        let ino = self.add_or_create_inode(&entry_path);

        match std::fs::rename(&entry_path, &newentry_path)
        {
            Err(e) => reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(()) =>
            {
                self.inode_to_path.insert(ino, newentry_path.as_os_str().to_os_string());
                self.path_to_inode.remove(entry_path.as_os_str());
                self.path_to_inode.insert(newentry_path.as_os_str().to_os_string(), ino);



                let mut table = TABLE!(self.table);
                if table.contains(&entry_path)
                {
                    table.rename(&newentry_path, &entry_path).expect("renaming");
                    table.flush().expect("flushing rename");
                }

                reply.ok();
            },
        }
    }

    fn link(&mut self, _req: &Request, ino: u64, newparent: u64, newname: &OsStr, reply: ReplyEntry)
    {
        // Not a true hardlink: new inode
        //println!("link");

        if !self.inode_to_path.contains_key(&ino)
        {
            return reply.error(ENOENT);
        }
        if !self.inode_to_path.contains_key(&newparent)
        {
            return reply.error(ENOENT);
        }

        let entry_path = Path::new(&self.inode_to_path[&ino]).to_owned();
        let newparent_path = Path::new(&self.inode_to_path[&newparent]);
        let newentry_path = newparent_path.join(newname);

        let newino = self.add_or_create_inode(&newentry_path);

        match std::fs::hard_link(&entry_path, &newentry_path)
        {
            Err(e) => reply.error(errhandle(e, || self.unregister_ino(ino))),
            Ok(()) =>
            {
                let attr = match std::fs::symlink_metadata(newentry_path)
                {
                    Err(e) =>
                    {
                        return reply.error(errhandle(e, || self.unregister_ino(newino)));
                    },
                    Ok(m) => meta2attr(&m, newino),
                };

                reply.entry(&TTL, &attr, 1);
            },
        }
    }

    fn mknod(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    )
    {
        //println!("mknod");
        // no mknod lib libstd
        reply.error(ENOSYS);
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    )
    {
        // Limited to setting file length only
        //println!("setattr");

        let (fh, sz) = match (fh, size)
        {
            (Some(x), Some(y)) => (x, y),
            _ =>
            {
                // only partial for chmod +x, and not the good one

                let entry_path = Path::new(&self.inode_to_path[&ino]).to_owned();

                if let Some(mode) = mode
                {
                    use std::fs::Permissions;

                    let perm = Permissions::from_mode(mode);
                    match std::fs::set_permissions(&entry_path, perm)
                    {
                        Err(e) => return reply.error(errhandle(e, || self.unregister_ino(ino))),
                        Ok(()) =>
                        {
                            let attr = match std::fs::symlink_metadata(entry_path)
                            {
                                Err(e) =>
                                {
                                    return reply.error(errhandle(e, || self.unregister_ino(ino)));
                                },
                                Ok(m) => meta2attr(&m, ino),
                            };

                            return reply.attr(&TTL, &attr);
                        },
                    }
                }
                else
                {
                    // Just try to do nothing, successfully.
                    let attr = match std::fs::symlink_metadata(entry_path)
                    {
                        Err(e) =>
                        {
                            return reply.error(errhandle(e, || self.unregister_ino(ino)));
                        },
                        Ok(m) => meta2attr(&m, ino),
                    };

                    return reply.attr(&TTL, &attr);
                }
            },
        };

        if !self.opened_files.contains_key(&fh)
        {
            return reply.error(EIO);
        }

        let f = self.opened_files.get_mut(&fh).unwrap();

        match f.set_len(sz)
        {
            Err(e) => reply.error(errhandle(e, || ())),
            Ok(()) =>
            {
                // pull regular file metadata out of thin air

                let attr = FileAttr {
                    ino,
                    size: sz,
                    blocks: 1,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                    blksize: BLOCK_SIZE,
                };

                reply.attr(&TTL, &attr);
            },
        }
    }

    fn setxattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        name: &OsStr,
        _value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    )
    {
        //println!("setxattr");
        let path = match self.inode_to_path.get(&_ino)
        {
            Some(f) => f,
            _ =>
            {
                return reply.error(2);
            },
        };

        match xattr::set(path, name, _value)
        {
            Ok(_) => reply.ok(),
            Err(_) => reply.error(2),
        }
    }

    // getxattr is a bit scuffed atm, look into this later..
    fn getxattr(&mut self, _req: &Request, _ino: u64, _name: &OsStr, size: u32, reply: ReplyXattr)
    {
        //println!("getxattr");
        let path = match self.inode_to_path.get(&_ino)
        {
            Some(f) => f,
            _ =>
            {
                return reply.error(2);
            },
        };

        match xattr::get(path, _name)
        {
            Ok(Some(data)) =>
            {
                if size == 0
                {
                    reply.size(data.len() as u32);
                }
                else if data.len() <= size as usize
                {
                    reply.data(&data);
                }
                else
                {
                    reply.error(libc::ERANGE);
                }
            },
            _ => reply.error(2),
        }
        /*let foo = xattr::get(path, _name).unwrap().unwrap();
        return reply.size(foo.len() as u32);*/
        // look into this !!!
        //reply.size(0);
    }

    fn destroy(&mut self)
    {
        println!("destroy");
    }
}
