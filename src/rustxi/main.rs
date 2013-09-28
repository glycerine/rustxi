/**
 *  copyright (c) 2013 Jason E. Aten and Do Nhat Minh
 *  license: the same as the Rust license options: dual MIT/Apache2.
 *
 *  rustxi: a revamp of rusti-the-repl using, where we
 *   use fork ping-ponging for transactional repl state maintenance.
 *
 *   See the README.md for details.
 *
 *
 **/

extern mod extra;

use std::{io, libc, os, vec, rt};
use callgraph::CallGraph;
use std::libc::{c_int, c_void};
use std::cast;

mod callgraph;
mod signum;
mod util;

static CODEBUF_SIZE: i64 = 4096;

// help(), banner(), prompt():
// generate user-facing help strings. Since these may be dynamic or
// localized or both, these need to be function calls not constants.
// Since prompt() is called by the ctrl-c signal handler, it must
// be global and not a method.
//
#[inline]
fn help() -> &str {
    static HELP: &'static str = "\
.?                   show help
.q                   exit rustxi
.h                   show line history
.c                   correct history only
.s file              source file -- XXTODO
.. {commands}        system(commands) -- XXTODO";

    HELP
}

#[inline]
fn banner() -> &str {
    static BANNER: &'static str = "rustxi: a transactional jit-based repl; \
                                   .? for help; .q or ctrl-d to exit.";
    BANNER
}

#[inline]
fn prompt() -> &str {
    static PROMPT: &'static str = "rustxi> ";
    PROMPT
}

#[fixed_stack_segment]
#[abi = "cdecl"]
fn ctrl_c_handler(_signum: c_int) {
    println(" [ctrl-c]");
    print(prompt());
}

#[fixed_stack_segment]
fn install_sigint_ctrl_c_handler() {
  unsafe { util::ll::signal(signum::SIGINT, cast::transmute(ctrl_c_handler)); }
}


struct Visor {
    /// history of commands
    cmd: ~[~str],

    // history of fail/success
    failed: ~[bool],

    /// function dependency graph
    callgraph: callgraph::BothWayGraph,
}

impl Visor {
    pub fn new() -> Visor {
        Visor{
            cmd: ~[],
            failed: ~[],
            callgraph: callgraph::BothWayGraph::new(),
        }
    }

    pub fn start(&mut self) {
        // only TRY should get SIGINT (ctrl-c)
        util::ignore_sigint();

        let visor_pid = util::getpid();
        let visor_sid = util::getsid(visor_pid);
        let visor_pgrp = util::getpgrp();

        // core dumping, for now commentout: install_sigint_ctrl_c_handler();

        debug!("visor called with pid:%?    sid:%?    pgrp:%?",
               visor_pid, visor_sid, visor_pgrp);

        //
        // setup fd to communicate
        // note that os.rs has Pipe{ input and out }, and the naming of
        // input and out may confusing; think of stdin and stdout in this case.
        // To use them you want to read from input, and write to out.
        //
        let pipe_code = os::pipe();
        let pipe_reply = os::pipe();

        // I'm visor
        let pid = util::fork();

        if pid > 0 {
            // I'm visor still.
            os::close(pipe_code.input);
            os::close(pipe_reply.out);

            println(banner());

            // READ LOOP: read code from stdin, send it on pipe_code
            loop {
                // cleanup zombies from when TRY succeeded and killed CUR
                let mut zombstatus :i32 = 0;
                util::waitpid_async(-1, &mut zombstatus);

                print(prompt());
                let code = io::stdin().read_line();

                let trimmed_code = code.trim();
                // match meta commands: keep these distinguished by the
                // first character for ease of typing and parsing.
                match trimmed_code {
                    "" if io::stdin().eof() => {
                        // ctrl-d should exit so we can send files on stdin eventually.
                        debug!("%d: VISOR: I see EOF", util::getpid() as int);
                        println("");

                        // send EOF on pipe_code to TRY, so it knows to shut itself down.
                        os::close(pipe_code.out);
                        os::close(pipe_reply.input);

                        // that's not working yet, so cleanup for sure with allquit().
                        util::process_group_exit();
                    },
                    "" => loop,
                    ".q" => util::process_group_exit(),
                    ".?" => {
                        println(help());
                        loop;
                    },
                    ".c" => {
                        // correct history only... failed commands commented out.
                        let mut i = 0;
                        for c in self.cmd.iter() {
                            if (self.failed[i]) { printf!("%s", "//not: ") }
                            printfln!("%s", *c);
                            i = i + 1;
                        }
                        loop;
                    },
                    ".h" => {
                        for c in self.cmd.iter() {
                            printfln!("%s", *c);
                        }
                        loop;
                    },
                    ".s" => {
                        printfln!("TODO: implement .s <file> sourcing.");
                        loop;
                    },
                    ".." => {
                        printfln!("TODO: implement system(cmd) shell outs.");
                        loop;
                    },
                    _ => {
                        // not a meta command: add to history
                        self.cmd.push(code.clone());
                    },
                }

                debug!("visor is: %?", self);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                vec::bytes::copy_memory(buffer, code.as_bytes(), code.len());
                buffer.truncate(code.len());
                //debug!("buffer is '%?' after copy from '%s'", buffer, code);

                // send code over to TRY
                do buffer.as_mut_buf |ptr, len| {
                    util::write(pipe_code.out, ptr as *libc::c_void, len as u64);
                }

                // wait for reply
                debug!("%d: I am VISOR: waiting for more, success, or failed",
                util::getpid() as int);
                // wait for "more" (from TRY) or "done" (from TRY) or "failed" (from CUR)
                let mut replybuf = ~[0u8, ..8];

                let bytesread = do replybuf.as_mut_buf |ptr, len| {
                    util::read(pipe_reply.input, ptr as *mut libc::c_void, len as u64)
                };

                if bytesread < 0 {
                    fail!("%d: I am VISOR: visor failed to read from code_pipe: %s",
                          util::getpid() as int,
                          os::last_os_error());
                }

                let replystr = do replybuf.as_mut_buf |ptr, _| {
                    util::copy_buf_to_string(ptr, bytesread as uint)
                };

                match replystr {
                    ~"failed"  => self.failed.push(true),
                    ~"success" => self.failed.push(false),
                    _ => fail!("VISOR doesn't recongize reply for CUR/TRY: %s", replystr),
                }

                debug!("%d: I am VISOR: I got an '%d' byte message back: '%s'",
                       util::getpid() as int,
                       bytesread as int, replystr);

            }
        } else {
            // I'm CUR after first fork, setup pipes on my end:
            os::close(pipe_code.out);
            os::close(pipe_reply.input);

            // TODO: needed? util::ll::rust_unset_sigprocmask();

        }

        // There are two processes that are descendants of VISOR: CUR and TRY.
        //
        // CUR holds the current state, in case it is needed for rollback.
        //  The first thing CUR does is spawn TRY.
        // TRY tries out the new code. If it finishes without fail!()-ing,
        //   then TRY replaces CUR.

        // steady-state: I'm CUR
        loop {
            util::ignore_sigint();

            debug!("%d: I am CUR: top of steady-state loop. About to fork a new TRY. parent: %d",
                   util::getpid() as int,
                   util::getppid() as int);

            let pid = util::fork();
            if pid == 0 {
                // I am TRY, child of CUR. I try new code out and succeed 
                // (and thence kill CUR and become CUR), or die.
                // TODO: needed? where? util::ll::rust_unset_sigprocmask();
                install_sigint_ctrl_c_handler();

                debug!("%d: I am TRY: about to request code line.",
                       util::getpid() as int);

                let mut buffer = ~[0u8, ..CODEBUF_SIZE];
                let mut bytes_read: i64;
                loop {
                    bytes_read = do buffer.as_mut_buf |ptr, len| {
                        util::read(pipe_code.input, ptr as *mut libc::c_void, len as u64)
                    };

                    if bytes_read < 0 {
                        debug!("read on pipe_code.out failed with errno: %? '%?'", 
                               os::errno(), os::last_os_error());
                        break;
                    }
                    if bytes_read > 0 { break; }
                }

                debug!("bytes_read is %d", bytes_read as int);
                buffer.truncate(bytes_read as uint);
                let code = do buffer.as_mut_buf |ptr, _| {
                    util::copy_buf_to_string(ptr, bytes_read as uint)
                };

                debug!("%d: TRY: I see code to run: '%s'", util::getpid() as int, code);
                /*
                 *  here is where call to do the majority of the
                 *  actual work: compile and run the code.
                 */
                self.compile_and_run_code_snippet(code);

                // we become the new CUR, so ignore ctrl-c again.
                util::ignore_sigint();
                debug!("%d: TRY succeeded in running the code, killing old CUR \
                        and I will become the new CUR.",
                        util::getpid() as int);
                let ppid = util::getppid();
                util::kill(ppid, libc::SIGTERM);

                // we are already a part of the visor's group, 
                // just we have init (pid 1) as a parent now.
                debug!("%d: TRY: I'm channeling Odysseus. I just killed ppid %d with SIGTERM.",
                       util::getpid() as int, ppid as int);

                pipe32reply("TRY", "success", pipe_reply.out);
            } else {
                // I am CUR. I wait for TRY to finish. If TRY succeeds I never
                // wake up. If TRY fails, I goto the
                // top of the steady-state loop and try again
                let mut status = 0 as c_int;
                util::waitpid(pid, &mut status);
                debug!("%d: CUR saw TRY process exit, must have failed. %s",
                       util::getpid() as int,
                       "Going to top of loop to spawn a new try.");

                // pipe "failed" to VISOR:
                pipe32reply("CUR", "failed", pipe_reply.out);
            }
        }


    } // end start()

    /**
     *  here is where the heart of the jit-repl will be: here
     *   we actually compile and run the code.
     **/
    fn compile_and_run_code_snippet(&mut self, code: &str) {
        match code.find_str(": ") {
            None => {
                debug!("%d: TRY: on code '%s', cannot find \": \"", 
                       util::getpid() as int, code);
                fail!("TRY code failure: parse error");
            },
            Some(pos) => {
                let func = code.slice_to(pos).to_owned();
                let deps: ~[&str] = code.slice_from(pos + 2).trim()
                    .split_iter(',').map(|s| s.trim())
                    .collect();
                let affected = self.callgraph.update(func, deps);
                for &f in affected.iter() {
                    print!("{:s} ", *f);
                }
                println("");
                debug!("%d: TRY: on code '%s', success.", 
                       util::getpid() as int, code);
            },
        }
    }
}

// reply with a message at most 32 bytes.
#[fixed_stack_segment]
fn pipe32reply(from: &str, replymsg: &str, fd: libc::c_int) -> i64 {
    static REPLYLEN: uint = 32;
    assert!(replymsg.len() < REPLYLEN);
    let mut replybuf = ~[0u8, ..REPLYLEN];
    vec::bytes::copy_memory(replybuf, replymsg.as_bytes(), replymsg.len());
    replybuf.truncate(replymsg.len());

    let mut bytes_written: i64;
    bytes_written = do replybuf.as_mut_buf |ptr, len| {
        util::write(fd, ptr as *libc::c_void, len as u64)
    };
    assert!(bytes_written == replymsg.len() as i64);

    if bytes_written < 0 {
        fail!("%d %s: read on pipe_code.out failed with errno: %? '%?'",
              util::getpid() as int, from, os::errno(), os::last_os_error());
    }

    debug!("%d: %s: sent replymsg of len '%?' with content '%s'",
           util::getpid() as int, from, replymsg.len(), replymsg);

    bytes_written
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
    // so we don't get extra threads.
    os::setenv("RUST_THREADS", "1");

    // and we ourselves run on the first thread.
    rt::start_on_main_thread(argc, argv, single_threaded_main)
}
