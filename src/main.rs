extern crate libc;
use libc::{_exit, STDIN_FILENO, STDOUT_FILENO};
use nix::sys::signal::*;
use nix::unistd::ForkResult::*;
use nix::{
    pty::*,
    sys::{signal, termios},
    unistd,
};
use std::ffi::{CString, OsStr};
use std::io::prelude::*;
use std::io::{stdin, stdout};
use std::os::unix::prelude::*;
use std::path::Path;

static SHELL: &str = "/home/taylor/repos/iot-shell/target/release/iot-shell";

fn main() {
    let shell = CString::new(SHELL).unwrap();
    let term = termios::tcgetattr(0).unwrap();
    let pty = forkpty(None, &term).unwrap();
    match pty.fork_result {
        Child => {
            let _res = unistd::execv(&shell, &[&shell]).unwrap();
            unreachable!();
        }
        Parent { child } => {

            let mut buffer = [0u8; 4096];
            let pid_path = Path.new(OsStr::new(format!("/proc/{}", child)));
                
                
            loop {

                loop {
                    let br = unistd::read(pty.master, &mut buffer);
                    print!("{}", std::str::from_utf8(&buffer).unwrap());
                    match br {
                        Ok(num_bytes) => match num_bytes {
                            0 => {
                                println!("bytes read: {}", num_bytes);
                                break;
                            },
                            _ => println!("bytes read: {}", num_bytes),
                        },
                        Err(err) => {
                            println!("Error: {:?}", err);
                            break;
                        }
                    }
                }
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                unistd::write(pty.master, input.as_bytes()).unwrap();
            }
            unistd::close(pty.master).unwrap();
        }
    }
}
