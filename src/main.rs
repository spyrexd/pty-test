extern crate libc;
use libc::{STDIN_FILENO, STDOUT_FILENO};
use nix::unistd::ForkResult::*;
use nix::{pty::*, sys::select::*, sys::signal::SigSet, sys::termios, sys::time::*, unistd};
use std::ffi::CString;
use std::os::unix::prelude::*;

static SHELL: &str = "/bin/sh";

fn set_tty_raw(fd: RawFd) -> nix::Result<()> {
    use nix::sys::termios::SpecialCharacterIndices::*;
    use nix::sys::termios::*;

    let mut term = tcgetattr(STDIN_FILENO)?;
    term.local_flags &=
        !(LocalFlags::ICANON | LocalFlags::ISIG | LocalFlags::IEXTEN | LocalFlags::ECHO);

    term.input_flags &= !(InputFlags::BRKINT
        | InputFlags::ICRNL
        | InputFlags::IGNBRK
        | InputFlags::IGNCR
        | InputFlags::INLCR
        | InputFlags::INPCK
        | InputFlags::ISTRIP
        | InputFlags::IXON
        | InputFlags::PARMRK);

    term.output_flags &= !OutputFlags::OPOST;

    term.control_chars[VMIN as usize] = 1;
    term.control_chars[VTIME as usize] = 0;

    tcsetattr(fd, SetArg::TCSAFLUSH, &term)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shell = CString::new(SHELL).unwrap();
    let orig_term = termios::tcgetattr(STDIN_FILENO)?;
    let pty = forkpty(None, &orig_term)?;
    match pty.fork_result {
        Child => {
            let _res = unistd::execv(&shell, &[&shell])?;
            unreachable!();
        }
        Parent { child } => {
            let mut buffer: [u8; 1024] = [0; 1024];

            set_tty_raw(STDIN_FILENO)?;

            loop {
                //select setup
                let mut fdset = FdSet::new();
                fdset.insert(STDIN_FILENO);
                fdset.insert(pty.master);

                let timeout = TimeSpec::seconds(10);
                let sigmask = SigSet::empty();

                pselect(None, &mut fdset, None, None, &timeout, &sigmask)?;

                if fdset.contains(STDIN_FILENO) {
                    let br = unistd::read(STDIN_FILENO, &mut buffer);
                    match br {
                        Ok(num_bytes) => {
                            unistd::write(pty.master, &mut buffer[0..num_bytes])?;
                        }
                        Err(_) => {
                            termios::tcsetattr(STDIN_FILENO, termios::SetArg::TCSANOW, &orig_term)?;
                            unistd::close(pty.master)?;
                            return Ok(());
                        }
                    };
                }

                if fdset.contains(pty.master) {
                    let br = unistd::read(pty.master, &mut buffer);
                    match br {
                        Ok(num_bytes) => {
                            unistd::write(STDOUT_FILENO, &buffer[0..num_bytes])?;
                        }
                        Err(_) => {
                            termios::tcsetattr(STDIN_FILENO, termios::SetArg::TCSANOW, &orig_term)?;
                            unistd::close(pty.master)?;
                            return Ok(());
                        }
                    };
                }
            }
        }
    };
}
