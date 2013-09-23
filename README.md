the rustxi repl
===============

     authors: Jason E. Aten, Ph.D. <j.e.aten@gmail.com> and Do Nhat Minh <m@minhdo.org>
     date: 21 Sept 2013
     copyright (c) 2013, Jason E. Aten and Do Nhat Minh
     license: the same as the Rust license options: dual MIT/Apache2.
 

rustxi is a next generation Rust REPL (read-eval-print-loop). It provides a transactional jit-compiled interpreter for the Rust language. If you make a mistake, you don't loose all your previous hard work. A syntax error, an assert!, or a fail! will not cause you to loose your accumulated data or history.

Rust + Transactions (commonly abbreviated TX) + Interpreter = rustxi


Rustxi Background
=======================

Status
------

Not done. This is an RFC + a spike of code to explore feasibility and semantics. The skeleton code in src/rustxi.rs implements the forkplan and demonstrates the interprocess communication framework between VISOR, CUR, and TRY successfully. This demonstrates feasibility.  However nothing is hooked up to rustc yet. The src/rustxi.rs code accepts your input, passes it to the TRY process upon request. TRY ignores the content of the lines you type, and instead simply flips a coin; actually it just declares success or failure based on whether the pid of TRY is even or odd. You can watch the process ids evolve on the left hand side of the printed debug output. If TRY fails, it dies and we rollback. If TRY succeeds, the old CUR process is replaced by the TRY process.

Executive summary
-----------------

Rustxi is a revamp of rusti-the-repl to provide transactional
rollback-on-fail!(). When you work in rustxi, you are isolated from two kinds of failures: failure
of the code to compile, and failures that happen at runtime when the code
is run. This means you can experiment freely at the rustxi repl.

Requirements: we require a single thread process image... so we can fork and have accurate and
efficient mistake-handling at the repl. Remember the goal is to rollback from any changes that have been made in the global process state during the execution of an arbitrary block of code.

In the code here I did a mini spike to evaluate ping-ponging between forked processes.

Outcome: implemented in src/rustxi.rs. Development and testing on linux. Works well. Feels snappy at the prompt. 

Conclusion: this is a very strong, robust approach.



Detailed architecture discussion
--------------------------------

There are three processes in the rustxi architecture: VISOR, CUR, and TRY.

First, the grandparent or VISOR -- exists mostly just to give a constant PID to monitor for rustxi. The VISOR lives as long as the rustxi session is going. The VISOR stores the history of commands executed so far. The VISOR accepts input from the user, and pipes it over to the CUR and TRY processes.

Then, there exist in rotation two other processes, two descendent processes of the VISOR. CUR holds the current state after all successful commands in the history have executed. The effects of any unsuccessful code snippets that were compiled and failed, or that were compiled and run and the failed, are completely invisible to CUR. TRY is the forked child of the current CUR, and is used to isolate all failure scenarios.


(0) In the beginning:

    Rustxi VISOR (forks off CUR)
       |
       |  fork
       |
      CUR (forks off TRY)
       |
       |  fork
       |
      TRY
    
    

    
(1)  Branching on success or fail!()ure: If the new code succeeds then TRY kills CUR, e.g. by doing kill(getppid(), SIGTERM);

     Rustxi VISOR
       |
      TRY
      
In detail: TRY, having suceeded (no fail! was called during compiling running the code snippet) kills CUR. CUR is no longer needed, so it dies, taking its old out-of-date state with it.

Status note aside: currently the part about TRY becoming a child of VISOR is fiction. We would *like* TRY to become the child of VISOR in the ps listings, but currently it is re-parented under init 1 because it's original parent CUR died when TRY killed it. This in no way changes the effectiveness of the approach. We diagram as if orphaned processes become children of VISOR because it simplifies and clarifies the explanation. All three processes do continue to be a part of the same process group.

Then TRY becomes the new CUR, here denoted CUR'. CUR' then in turn forks a new repl, TRY', and we goto 0. to begin again, looking like this:

    Rustxi VISOR
      |
     CUR'
      |
      | fork
      |
     TRY'

    
(2) If the new code in TRY fails, then CUR recieves SIGCHLD:

     Rustxi VISOR
       |
      CUR

Detail: TRY when testing the new code, failed. hopefully TRY printed an appropriate error message. Optionally we could start/attach gdb (or even be running under gdb already?). In any case, once the optional debug step is done, CUR notes the failure by receiving/handling SIGCHLD, and prints a failure message itself just in case it wasn't already obvious. Then CUR forks a new child, TRY', and we goto 0. to begin again, looking like this:

    Rustxi VISOR
      |
     CUR 
      |
      | fork
      |
     TRY'


Summary: 
--------
In this architecture, CUR is the mediator between VISOR and TRY. The purpose of using processes is that we can have inexpensive commit and rollback on failure/fail!() in the already-jitted and now-we-are-running it code. Since the jitted code may make calls into any pre-compiled library and hence make arbitrary changes to the global process state, fork is the only sane way to rollback.

// Additional (nice) option: start gdb on failure of process, so we can view stack traces.

Discussion/aesthetics
-------------------------

I like the fork(2) approach because it provides transactional semantics which means that rustxi can be relied upon to not loose my work.

* pluses

 + it avoids (and requires avoiding) threading. This is a huge win, in my opinion.  Too many projects have fallen into the deep dark pit of threads. During development, you want deterministic behavior, not threads.

 + it leverages the hardware Memory Management Unit and virtual memory support from the kernel, so we don't have to reimplement transactions (slow to run and painful to do so, and will be far from comprehensive). The design using fork gives us fast and comprehensive rollback. If we call into C code that manipulates global variables, these get rolled back. If we close or open file handles, these get rolled back. If we spawn or kill rust coroutines (tasks) on this thread thread, these will get rolled back. Using fork is a fairly comprehensive solution, since it has been tuned under the kernel for years. 

* minuses. Possible disadvantages of this approach:

 + fork only works if you only ever have one thread.  Not a problem, since this is what sanity during development wants anyway. It does mean rustxi cannot be an exact replica of fully-threaded rustc-produced binary semantics. Rustxi cannot be comprehensive. That is okay. Comprehensiveness is a non-goal. We value 80% of the win for 20% the effort.


* observation: 

 + If the rust runtime provided a synchronization barrier checkin-point for all threads, some kind of call that all threads were required to cooperatively call once in while, then multiple threads could also be supported under fork. Otherwise fork will eventually copy a process while some thread is holding the malloc mutex, at which point the forked copy will deadlock on memory allocation.







about Rust the language
-----------------------

Rust is a modern langauge from Mozilla Research. It has  support for 
 writing embedded applications that are memory safe and simultaneously
 do not suffer garbage-collection pauses. license: dual MIT / Apache 2.

 You'll want the github MASTER branch of rust to do anything useful
 and up-to-date. The project has strong velocity, so it is evolving
 quickly.  This code was developed under rustc at the following point.
 To try the src/rustxi.rs code, you will want a compiler at least this new:

 * rustc 0.8-pre (570431f 2013-09-19 15:56:04 -0700)

- http://www.rust-lang.org/
- https://github.com/mozilla/rust

