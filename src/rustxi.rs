/**
*  copyright (c) 2013 Jason E. Aten and Do Nhat Minh
*  license: the same as the Rust license options: dual MIT/Apache2.

*  rustxi: a revamp of rusti-the-repl using, where we
*   explore fork ping-ponging for repl state maintenance
*   in the case of user syntax error or runtime fail!().
*
*   See the README.md for detils. 
*
* 
**/


extern mod std;

//use std::libc::size_t;
// use std::libc::sleep;
//use std::libc::funcs::posix88::unistd::fork;
use std::cast;
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
    use std::libc::types::common::c95::{FILE};
    
    extern {
        pub fn kill(pid: pid_t, sig: c_int) -> c_int;
        pub fn getsid(pid: pid_t) -> c_int;
        pub fn getpgrp() -> c_int;
        pub fn setpgid(pid: pid_t, pgid: pid_t) -> c_int;
        pub fn signal(signum: c_int, handler: i64);
        pub fn clearerr(fd : *FILE);
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

#[fixed_stack_segment]
#[abi = "cdecl"]
fn ctrl_c_handler(_signum: c_int) {
  printfln!("%s"," [ctrl-c]");
}

#[fixed_stack_segment]
fn install_sigint_ctrl_c_handler() {
  unsafe { my_c::signal(signum::SIGINT, cast::transmute(&ctrl_c_handler)); }
}


static CODEBUF_SIZE : i64 = 4096;

// help
fn help() -> ~str {
  ":help    show this help\n" +
  ":quit    exit rustxi"
}

fn banner() -> &str {
  "rustxi: a transactional jit-based repl. :help for help; :quit or ctrl-d to exit."
}

// reply with a message at most 32 bytes.
#[fixed_stack_segment]
fn pipe32reply(from: &str, replymsg: &str, fd: c_int) -> i64 {
  
  static REPLYLEN : uint = 32;
  assert!(replymsg.len() < REPLYLEN);
  let mut replybuf = ~[0u8, ..REPLYLEN];
  std::vec::bytes::copy_memory(replybuf, replymsg.as_bytes(), replymsg.len());
  replybuf.truncate(replymsg.len());
  
  let mut bytes_written : i64;
  bytes_written = do replybuf.as_mut_buf |ptr, len| {
      unsafe {
	std::libc::write(fd, ptr as *std::libc::types::common::c95::c_void, len as u64)
      }
    };
  assert!(bytes_written == replymsg.len() as i64);
  
  if (bytes_written < 0) {
    fail!("%d %s: read on pipe_code.out failed with errno: %? '%?'", getpid() as int, from, std::os::errno(), std::os::last_os_error());
  }
  
  debug!("%d %s: sent replymsg of len '%?' with content '%s'", getpid() as int, from, replymsg.len(), replymsg);

  bytes_written
}	



impl Visor {

    pub fn new() -> Visor {
        Visor{cmd: ~[]}
    }

    pub fn allquit(&mut self) {
      #[fixed_stack_segment]; #[inline(never)];
      unsafe { 
	// send SIGTERM to all processes in my process group
	my_c::kill(0, std::libc::SIGTERM);
	exit(0); 
      }
    }


    pub fn start(&mut self) {
        #[fixed_stack_segment]; #[inline(never)];

        use std::libc::funcs::posix01::wait::*;

	install_sigint_ctrl_c_handler();

        mod rustrt {
            #[abi = "cdecl"]
            extern {
                pub fn rust_unset_sigprocmask();
            }
        }

        // only TRY should get SIGINT (ctrl-c)
	//        ignore_sigint();

        let visor_pid : c_int = getpid();
        let visor_sid : c_int = getsid(visor_pid);
        let visor_pgrp : c_int = getpgrp();
        
        
        debug!("visor called with pid:%?    sid:%?    pgrp:%?", visor_pid, visor_sid, visor_pgrp);
        
        // setup fd to communicate
        // note that os.rs has Pipe{ input and out } backwards, so
        //  to use them, we have to use them backwards.
        // correct order would be {out, input}. But os.rs lists {input, out}.
        // 
        let pipe_code = std::os::pipe();
        let pipe_reply = std::os::pipe();

        //debug!("visor: my pipe_code is %?", pipe_code);
        
        // I'm visor
        let pid = unsafe { fork() };
        if (pid < 0) { fail!("rustxi visor failure in fork: %s", std::os::last_os_error()); }
        if (pid > 0) {

            // I'm visor still.
	    std::os::close(pipe_code.input);
	    std::os::close(pipe_reply.out);

	    println(banner());

            // READ LOOP: read code from stdin, send it on pipe_code
            while(true) {
                
                // cleanup zombies from when TRY succeeded and killed CUR
                let mut zombstatus :i32 = 0;
                //let chpid = 
                unsafe {
                    std::libc::funcs::posix01::wait::waitpid(-1, &mut zombstatus, WNOHANG)
                };

	        printf!("%s","rustxi> ");
	        let code : ~str = stdin().read_line();
                
                self.cmd.push(code.clone());

                debug!("visor is: %?", self);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                std::vec::bytes::copy_memory(buffer, code.as_bytes(), code.len());
                buffer.truncate(code.len());
                //debug!("buffer is '%?' after copy from '%s'", buffer, code);

		let trimcode = code.trim();
		if ("".equiv(&trimcode) && stdin().eof()) {
		  debug!("%d: VISOR: I see EOF", getpid() as int);
		  println("");

		  // send EOF on pipe_code to TRY, so it knows to shut itself down.
		  std::os::close(pipe_code.out);
		  std::os::close(pipe_reply.input);

		  // that's not working yet, so cleanup for sure with allquit().
		  self.allquit();
		  unsafe { exit(0); }
		}
		else
		if ("".equiv(&trimcode)) { loop; }
		else
		if (":quit".equiv(&trimcode)) {
		  self.allquit();
		}
		else
		if (":help".equiv(&trimcode)) {
		  println(help());
		  loop;
		}

		// send code over to TRY
                do buffer.as_mut_buf |ptr, len| {
	            unsafe {
                        std::libc::write(pipe_code.out, ptr as *std::libc::types::common::c95::c_void, len as u64);
                    }
                }

		// wait for reply
	        debug!("%d: I am VISOR: waiting for more, success, or failed", getpid() as int);
		// wait for "more" (from TRY) or "done" (from TRY) or "failed" (from CUR)
                let mut replybuf = ~[0u8, ..8];

                let bytesread = do replybuf.as_mut_buf |ptr, len| {
		    unsafe {
		      std::libc::read(pipe_reply.input, ptr as *mut std::libc::types::common::c95::c_void, len as u64)
		    }
		  };

		if (bytesread < 0) {
		  fail!("%d: I am VISOR: visor failed to read from code_pipe: %s", getpid() as int, std::os::last_os_error());
		}

                let replystr = do replybuf.as_mut_buf |ptr, _| {
                    copy_buf_to_string(ptr, bytesread as uint)
                };

		debug!("%d: I am VISOR: I got an '%d' byte message back: '%s'", getpid() as int, bytesread as int, replystr);


            }
            
        } else {
            // I'm CUR after first fork, setup pipes on my end:
	    std::os::close(pipe_code.out);
	    std::os::close(pipe_reply.input);
	    //println!("");
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

	    debug!("%d: I am CUR: top of steady-state loop. About to fork a new TRY. parent: %d", getpid() as int, getppid() as int);

	    let pid = unsafe { fork() };
	    if (pid < 0) { fail!("rustxi visor failure in fork: %s", std::os::last_os_error()); }
	    if (0 == pid) {
	        // I am TRY, child of CUR. I try new code out and succeed (and thence kill CUR and become CUR), or die.

                unsafe { rustrt::rust_unset_sigprocmask(); }
                deliver_sigint();

	        debug!("%d: I am TRY: about to request code line.", getpid() as int);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                let mut bytes_read : i64 = -1;
                while (true) {
                    bytes_read = do buffer.as_mut_buf |ptr, len| {
	                unsafe {
                            std::libc::read(pipe_code.input, ptr as *mut std::libc::types::common::c95::c_void, len as u64)
                        }
                    };

                    if (bytes_read < 0) {
                        fail!("read on pipe_code.out failed with errno: %? '%?'", std::os::errno(), std::os::last_os_error());
                        //debug!("read on pipe_code.out failed with errno: %? '%?'", std::os::errno(), std::os::last_os_error());
			// break;
                    }
                    if (bytes_read > 0) { break; }
                }

                debug!("bytes_read is %d", bytes_read as int);
                buffer.truncate(bytes_read as uint);
                //              let code = std::str::from_utf8(buffer);
                let code = do buffer.as_mut_buf |ptr, _| {
                    copy_buf_to_string(ptr, bytes_read as uint)
                };

	        debug!("%d: TRY: I see code to run: '%s'", getpid() as int, code);
	        /*
	        *  here is where call to do the majority of the
                *  actual work: compile and run the code.
	        */
                compile_and_run_code_snippet(code);


                // we become the new CUR, so ignore ctrl-c again.
                ignore_sigint();
	        debug!("%d: TRY succeeded in running the code, killing old CUR and I will become the new CUR.", 
		          getpid() as int);
                let ppid = getppid();
	        unsafe { my_c::kill(ppid, std::libc::SIGTERM);  }

                // we are already a part of the visor's group, just we have init (pid 1) as a parent now.
	        debug!("%d: TRY: I'm channeling Odysseus. I just killed ppid %d with SIGTERM.",
		          getpid() as int, ppid as int);

		pipe32reply("TRY", "success", pipe_reply.out);

	    } else {
	        // I am CUR. I wait for TRY to finish. If TRY succeeds I never wake up. If TRY fails, I goto the
	        // top of the steady-state loop and try again
	        std::run::waitpid(pid);
	        debug!("%d: CUR saw TRY process exit, must have failed. Going to top of loop to spawn a new try.", 
		          getpid() as int);

		// pipe "failed" to VISOR:

		pipe32reply("CUR", "failed", pipe_reply.out);
	
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



/**
*  here is where the heart of the jit-repl will be: here
*   we actually compile and run the code.
**/
#[fixed_stack_segment]
fn compile_and_run_code_snippet(code : &str) {

    // for now, simulate failure half the time.
    if (getpid() % 2 == 0) {
	debug!("%d: TRY: on code '%s', simulating fail!", getpid() as int, code);
        fail!("%d: TRY code failure simulated here with fail!()", getpid() as int);
    }

    debug!("%d: TRY: on code '%s', simulating success.", getpid() as int, code);

}


#[fixed_stack_segment]
fn setup_ctrl_c_handler() {
  /*
    struct sigaction sa;
    bzero(&sa,sizeof(struct sigaction));
    struct sigaction oldact;
    sa.sa_handler = &ctrl_c_handler;
    if (-1 == sigaction(SIGINT , &sa, &oldact)) {
        perror("error: could not setup SIGINT signal handler. Aborting.");
        exit(1);
    }
  */
}

