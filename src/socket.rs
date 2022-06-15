use std::{
    io::{self, Read},
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use crate::{table::Table, TABLE};

fn read_stream(stream: &mut TcpStream, buf: &mut [u8], exit_condition: &Arc<AtomicBool>) -> bool
{
    loop
    {
        match stream.read_exact(buf)
        {
            Ok(()) =>
            {
                return true;
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
            {
                if exit_condition.load(Ordering::Relaxed)
                {
                    return false;
                }
                continue;
            },
            Err(_) => return false,
        };
    }
}

//fn parse_message<'a>(s: &'a str) -> Option<(&'a str,

fn handle_message(s: &str, state: &Arc<Mutex<Table>>)
{
    let s = s.trim();
    let mut iter = s.split_ascii_whitespace();
    match (iter.next(), iter.next())
    {
        (Some(statement), Some(dataset)) =>
        {
            if statement == "revoke"
            {
                let mut table = TABLE!(state);
                table.revoke(dataset).expect("revoking");
                table.flush().expect("flushing");
            }
        },
        _ =>
        {},
    };
}


pub fn spawn(exit_condition: Arc<AtomicBool>, state: Arc<Mutex<Table>>)
{
    let mut stream: TcpStream = loop
    {
        if let Ok(stream) = TcpStream::connect("0.0.0.0:58642")
        {
            break stream;
        }

        if exit_condition.load(Ordering::Relaxed)
        {
            std::process::exit(0);
        }

        std::thread::sleep(std::time::Duration::from_millis(1000));
    };

    stream.set_nonblocking(true).expect("set_nonblocking call failed");

    loop
    {
        let mut len: [u8; 4] = [0; 4];
        if !read_stream(&mut stream, &mut len, &exit_condition)
        {
            exit_condition.store(true, Ordering::Relaxed);
            return;
        }
        let message_length = u32::from_be_bytes(len);

        let mut buf = vec![0; message_length as usize];

        if !read_stream(&mut stream, buf.as_mut_slice(), &exit_condition)
        {
            exit_condition.store(true, Ordering::Relaxed);
            return;
        }

        let s = std::str::from_utf8(&buf).expect("turing into str");
        handle_message(s, &state);
    }
}
