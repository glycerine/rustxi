/**
*  rustxi: a revamp of rusti-the-repl using fork.
*
*  rustxi.rs : explore fork ping-ponging for repl state maintenance
*   in the case of user error.
*
*  author: Jason E. Aten <j.e.aten@gmail.com>
*  date: 21 Sept 2013
*  copyright (c) 2013, Jason E. Aten
*  license: the same as the Rust license options: dual MIT/Apache2.
* 
*  We want a single thread... so we can fork and have accurate and
*  efficient mistake-handling at the repl. Remember the goal is to
*  rollback from any changes that global.
*
*  So here I did a mini spike to evaluate ping-ponging between forked processes.
*
*  Outcome: implemented below. Works well. Feels snappy at the prompt. 
*
*  Conclusion: this is a very strong, robust approach.
*
**/


/** 
Detailed architecture discussion:

There are three processes in the robust, transactional rustxi architecture: VISOR, CUR, and TRY.

First, the grandparent or VISOR -- exists mostly just to give a constant PID to monitor for rustxi. The VISOR lives as long as the rustxi session is going. The VISOR stores the history of commands executed so far. The VISOR accepts input from the user, and pipes it over to CUR.

Then, there exist in rotation two other processes, two descendent processes of the VISOR. CUR holds the current state after all successful commands in the history have executed. The effects of any unsuccessful code snippets that were compiled and failed, or that were compiled and run and the failed, are completely invisible to CUR. TRY is the forked child of the current CUR, and is used to isolate all failure scenarios.


0. the beginning:

Rustxi VISOR
|
CUR (forks off TRY)
|
|  fork(2)
|
TRY


Branching:

1. If the new code succeeds then TRY kills CUR, e.g. by doing kill(getppid(), SIGTERM);

Rustxi VISOR
|
TRY

In detail: TRY, having suceeded (no fail! was called during compiling running the code snippet) kills CUR. CUR is no longer needed, so it dies, taking its old out-of-date state with it.

Then TRY becomes the new CUR, here denoted CUR'. CUR' then in turn forks a new repl, TRY', and we goto 0. to begin again, looking like this:

Rustxi VISOR
|
CUR'
|
| fork(2)
|
TRY'


2. If the new code in TRY fails, then CUR recieves SIGCHLD:

Rustxi VISOR
|
CUR

Detail: TRY when testing the new code, failed. hopefully TRY printed an appropriate error message. Optionally we could start/attach gdb (or even be running under gdb already?). In any case, once the optional debug step is done, CUR notes the failure by receiving/handling SIGCHLD, and prints a failure message itself just in case it wasn't already obvious. Then CUR forks a new child, TRY', and we goto 0. to begin again, looking like this:

Rustxi VISOR
|
CUR 
|
| fork(2)
|
TRY'

Summary: In this architecture, CUR is the mediator between VISOR and TRY. The purpose of using processes is that we can have inexpensive commit and rollback on failure/fail!() in the already-jitted and now-we-are-running it code. Since the jitted code may make calls into any pre-compiled library and hence make arbitrary changes to the global process state, fork is the only sane way to rollback.

// Additional (nice) option: start gdb on failure of process, so we can view stack traces.

Discussion:

I like the fork(2) approach because

+ it avoids (and requires avoiding) threading. This is a huge win, in my opinion.  Too many projects have fallen into the deep dark pit of threads. During development, you want deterministic behavior, not threads.

+ it leverages the hardware Memory Management Unit and virtual memory support from the kernel, so we don't have to reimplement transactions (slow to run and painful to do so, and will be far from comprehensive). The design using fork gives us fast and comprehensive rollback. If we call into C code that manipulates global variables, these get rolled back. If we close or open file handles, these get rolled back. If we have spawn or kill rust coroutines (tasks) on this same thread, these will get rolled back. Using fork is a fairly comprehensive solution, since it has been tuned under the kernel for years.

Possible disadvantages of this approach:
- fork only works if you only ever have one thread.  Not a problem, since this is what sanity during development wants anyway. But this will constaint rustxi to not be comprehensive. Comprehensiveness is a non-goal anyway, so this is okay. 80/20 applies.

**/


extern mod std;

//use std::libc::size_t;
// use std::libc::sleep;
//use std::libc::funcs::posix88::unistd::fork;
use std::libc::*;
use std::os::*;
use std::io::stdin;
//use std::run::*;
use signum::*;

mod signum;

pub static WNOHANG: c_int = 1;

#[nolink]
#[abi = "cdecl"]
pub mod my_c {
    use std::libc::types::os::arch::c95::{c_int};
    use std::libc::types::os::arch::posix88::{pid_t};
    
    extern {
        pub fn kill(pid: pid_t, sig: c_int) -> c_int;
        pub fn getsid(pid: pid_t) -> c_int;
        pub fn getpgrp() -> c_int;
        pub fn setpgid(pid: pid_t, pgid: pid_t) -> c_int;
        pub fn signal(signum: c_int, handler: i64);
    }
}



pub fn copy_buf_to_string(buf: *mut u8, len: uint) -> ~str {
    unsafe { std::str::raw::from_buf_len(buf as *u8, len) }
}


pub struct Visor {

    // the history of commands
    cmd:  ~[~str],
}

// utility functions from libc

#[fixed_stack_segment]
fn getpid() -> c_int {
    unsafe { std::libc::getpid() }
}

#[fixed_stack_segment]
fn getppid() -> c_int {
    unsafe { std::libc::getppid() }
}

#[fixed_stack_segment]
fn getsid(pid : c_int) -> c_int {
    unsafe { my_c::getsid(pid) }
}

#[fixed_stack_segment]
fn getpgrp() -> c_int {
    unsafe { my_c::getpgrp() }
}

#[fixed_stack_segment]
fn ignore_sigint() {
    unsafe { my_c::signal(signum::SIGINT, signum::SIG_IGN as i64); }
}

#[fixed_stack_segment]
fn deliver_sigint() {
    unsafe { my_c::signal(signum::SIGINT, signum::SIG_DFL as i64); }
}


static CODEBUF_SIZE : i64 = 4096;


impl Visor {

    pub fn new() -> Visor {
        Visor{cmd: ~[]}
    }


    pub fn start(&mut self) {
        #[fixed_stack_segment]; #[inline(never)];

        use std::libc::funcs::posix01::wait::*;

        mod rustrt {
            #[abi = "cdecl"]
            extern {
                pub fn rust_unset_sigprocmask();
            }
        }

        // only TRY should get SIGINT (ctrl-c)
        ignore_sigint();

        let visor_pid : c_int = getpid();
        let visor_sid : c_int = getsid(visor_pid);
        let visor_pgrp : c_int = getpgrp();
        
        
        printfln!("visor called with pid:%?    sid:%?    pgrp:%?", visor_pid, visor_sid, visor_pgrp);
        
        // setup fd to communicate
        // note that os.rs has Pipe{ input and out } backwards, so
        //  to use them, we have to use them backwards.
        // correct order would be {out, input}. But os.rs lists {input, out}.
        // 
        let pipe_code = std::os::pipe();
        

        printfln!("my pipe_code is %?", pipe_code);
        
        // I'm visor
        let pid = unsafe { fork() };
        if (pid < 0) { fail!("rustxi visor failure in fork: %s", std::os::last_os_error()); }
        if (pid > 0) {

            // I'm visor still.
            std::os::close(pipe_code.input);

            // READ LOOP: read code from stdin, send it on pipe_code
            while(true) {
                
                // cleanup zombies from when TRY succeeded and killed CUR
                let mut zombstatus :i32 = 0;
                //let chpid = 
                unsafe {
                    std::libc::funcs::posix01::wait::waitpid(-1, &mut zombstatus, WNOHANG)
                };

	        printf!("%s","<rustxi use :exit to quit> ");
	        let code : ~str = stdin().read_line();
                
                self.cmd.push(code.clone());

                printfln!("visor is: %?", self);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                std::vec::bytes::copy_memory(buffer, code.as_bytes(), code.len());
                buffer.truncate(code.len());
                printfln!("buffer is '%?' after copy from '%s'", buffer, code);

		if (":exit".equiv(&code)) {
		  println("[rustxi done]");
		  unsafe { exit(0); }
		}

                do buffer.as_mut_buf |ptr, len| {
	            unsafe {
                        std::libc::write(pipe_code.out, ptr as *std::libc::types::common::c95::c_void, len as u64);
                    }
                }
            }
            
        } else {
            // I'm CUR after first fork, setup pipes on my end:
            std::os::close(pipe_code.out);
            println!("");
            unsafe { rustrt::rust_unset_sigprocmask(); }
        }
	
	// There are two processes that are descendants of VISOR: CUR and TRY.
	//
	// CUR holds the current state, in case it is needed for rollback.
	//  The first thing CUR does is spawn TRY.
	// TRY tries out the new code. If it finishes without fail!()-ing,
	//   then TRY replaces CUR.

        // steady-state: I'm CUR
	while(true) {
            ignore_sigint();              

	    printfln!("%d: I am CUR: top of steady-state loop. About to fork a new TRY. parent: %d", getpid() as int, getppid() as int);

	    let pid = unsafe { fork() };
	    if (pid < 0) { fail!("rustxi visor failure in fork: %s", std::os::last_os_error()); }
	    if (0 == pid) {
	        // I am TRY, child of CUR. I try new code out and succeed (and thence kill CUR and become CUR), or die.

                unsafe { rustrt::rust_unset_sigprocmask(); }
                deliver_sigint();

	        printfln!("%d: I am TRY: about to request code line. pipecode.input = %d", getpid() as int, pipe_code.input as int);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                let mut bytes_read : i64 = -1;
                while (true) {
                    bytes_read = do buffer.as_mut_buf |ptr, len| {
	                unsafe {
                            std::libc::read(pipe_code.input, ptr as *mut std::libc::types::common::c95::c_void, len as u64)
                        }
                    };

                    if (bytes_read < 0) {
                        printfln!("read on pipe_code.out failed with errno: %? '%?'", std::os::errno(), std::os::last_os_error());
                        break;
                    }
                    if (bytes_read > 0) { break; }
                }

                printfln!("bytes_read is %d", bytes_read as int);
                buffer.truncate(bytes_read as uint);
                //              let code = std::str::from_utf8(buffer);
                let code = do buffer.as_mut_buf |ptr, _| {
                    copy_buf_to_string(ptr, bytes_read as uint)
                };

	        printfln!("%d: TRY: I see code to run: '%s'", getpid() as int, code);
	        /*
	        *  here is where call to do the majority of the
                *  actual work: compile and run the code.
	        */
                compile_and_run_code_snippet(code);


                // we become the new CUR, so ignore ctrl-c again.
                ignore_sigint();              
	        printfln!("%d: TRY succeeded in running the code, killing old CUR and I will become the new CUR.", 
		          getpid() as int);
                let ppid = getppid();
	        unsafe { my_c::kill(ppid, std::libc::SIGTERM);  }

                // we are already a part of the visor's group, just we have init (pid 1) as a parent now.
	        printfln!("%d: TRY: I'm channeling Odysseus. I just killed ppid %d with SIGTERM.",
		          getpid() as int, ppid as int);

	    } else {
	        // I am CUR. I wait for TRY to finish. If TRY succeeds I never wake up. If TRY fails, I goto the
	        // top of the steady-state loop and try again
	        std::run::waitpid(pid);
	        printfln!("%d: CUR saw TRY process exit, must have failed. Going to top of loop to spawn a new try.", 
		          getpid() as int);
	    }
	}
	
	
    } // end start()

}



#[fixed_stack_segment]
fn single_threaded_main() {
    let mut v = Visor::new();
    v.start();
}


// if you want to be sure you are running on the main thread, do this:
#[start]
#[fixed_stack_segment]
fn start(argc: int, argv: **u8) -> int {
    std::rt::start_on_main_thread(argc, argv, single_threaded_main)
}


#[cfg(unix)]
fn myfork() -> i32 {
    #[fixed_stack_segment]; #[inline(never)];

    mod rustrt {
        #[abi = "cdecl"]
        extern {
            pub fn rust_unset_sigprocmask();
        }
    }

    unsafe {

        let pid = fork();
        if pid < 0 {
            fail!("failure in fork: %s", std::os::last_os_error());
        } else if pid > 0 {
            printfln!("parent sees pid >0: child is %d. Parent is %d", pid as int, std::libc::getpid() as int);
            return pid;
        }

        printfln!("child sees pid == 0: child is %d, actual pid is %d", pid as int, std::libc::getpid() as int);

        rustrt::rust_unset_sigprocmask();

	return pid;
    }
}

/**
*  here is where the heart of the jit-repl will be: here
*   we actually compile and run the code.
**/
#[fixed_stack_segment]
fn compile_and_run_code_snippet(code : &str) {

    // for now, simulate failure half the time.
    if (getpid() % 2 == 0) {
	printfln!("%d: TRY: on code '%s', simulating fail!", getpid() as int, code);
        fail!("TRY code failure simulated here with fail!()");
    }

    printfln!("%d: TRY: on code '%s', simulating success.", getpid() as int, code);

}

