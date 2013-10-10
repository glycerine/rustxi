use std::{libc, os, str};
use std::libc::{c_int, c_void};
use std::libc::types::os::arch::posix88::{pid_t};

mod signum;

#[nolink]
#[abi = "cdecl"]
pub mod ll {
    use std::libc::{c_int, c_void, c_schar};
    use std::libc::types::os::arch::posix88::{pid_t};
    use std::libc::types::common::c95::{FILE};

    extern {
        pub fn kill(pid: pid_t, sig: c_int) -> c_int;
        pub fn getsid(pid: pid_t) -> c_int;
        pub fn getpgrp() -> c_int;
        pub fn setpgid(pid: pid_t, pgid: pid_t) -> c_int;
        pub fn signal(signum: c_int, handler: *c_void);
        pub fn rust_unset_sigprocmask();
        pub fn clearerr(fd : *FILE);
        pub fn getenv(name: *mut c_schar) -> *c_schar;
        pub fn setenv(name: *c_schar, value: *c_schar, overwrite: c_int) -> c_int;
        pub fn unsetenv(name: *c_schar) -> c_int;
    }
}


#[fixed_stack_segment]
pub fn waitpid(pid: libc::pid_t, status: &mut libc::c_int) -> libc::pid_t {
    unsafe { libc::funcs::posix01::wait::waitpid(pid, status, 0) }
}

#[fixed_stack_segment]
pub fn waitpid_async(pid: libc::pid_t, status: &mut libc::c_int) -> libc::pid_t {
    unsafe { libc::funcs::posix01::wait::waitpid(pid, status, 1) }
}

#[fixed_stack_segment]
pub fn getpid() -> libc::pid_t {
    unsafe { libc::getpid() }
}

#[fixed_stack_segment]
pub fn getppid() -> libc::pid_t {
    unsafe { libc::getppid() }
}

#[fixed_stack_segment]
pub fn getsid(pid: libc::pid_t) -> libc::pid_t {
    unsafe { ll::getsid(pid) }
}

#[fixed_stack_segment]
pub fn getpgrp() -> libc::pid_t {
    unsafe { ll::getpgrp() }
}

#[fixed_stack_segment]
pub fn ignore_sigint() {
    unsafe { ll::signal(signum::SIGINT, signum::SIG_IGN); }
}

#[fixed_stack_segment]
pub fn deliver_sigint() {
    unsafe { ll::signal(signum::SIGINT, signum::SIG_DFL); }
}

#[fixed_stack_segment]
pub fn kill(pid: libc::pid_t, sig: libc::c_int) -> libc::c_int {
    unsafe { ll::kill(pid, sig) }
}

pub fn copy_buf_to_string(buf: *mut u8, len: uint) -> ~str {
    unsafe { str::raw::from_buf_len(buf as *u8, len) }
}

#[cfg(unix)]
#[fixed_stack_segment]
pub fn fork() -> libc::pid_t {
    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            fail!("failure in fork: %s", os::last_os_error());
        }
        pid
    }
}

#[fixed_stack_segment]
pub fn exit(status: libc::c_int) -> ! {
    unsafe { libc::exit(status) }
}

#[fixed_stack_segment]
pub fn read(fd: libc::c_int, buf: *mut libc::c_void, count: libc::size_t) -> libc::ssize_t {
    unsafe { libc::read(fd, buf, count) }
}

#[fixed_stack_segment]
pub fn write(fd: libc::c_int, buf: *libc::c_void, count: libc::size_t) -> libc::ssize_t {
    unsafe { libc::write(fd, buf, count) }
}

pub fn process_group_exit() -> ! {
    #[fixed_stack_segment]; #[inline(never)];
    unsafe { 
        // send SIGTERM to all processes in my process group
        ll::kill(0, libc::SIGTERM);
        libc::exit(0); 
    }
}
