use std::libc::{c_int, c_void};

// linux signal numbers, from
//  /usr/include/x86_64-linux-gnu/bits/signum.h

    pub static  SIG_ERR   : *c_void = -1 as *c_void;   /* Error return.   */
    pub static  SIG_DFL   : *c_void = 0 as *c_void;    /* Default action. */
    pub static  SIG_IGN   : *c_void = 1 as *c_void;    /* Ignore signal.  */

    /* Signals.  */
    pub static  SIGHUP    : c_int = 1;       /* Hangup (POSIX).  */
    pub static  SIGINT    : c_int = 2;       /* Interrupt (ANSI).  */
    pub static  SIGQUIT   : c_int = 3;       /* Quit (POSIX).  */
    pub static  SIGILL    : c_int = 4;       /* Illegal instruction (ANSI).  */
    pub static  SIGTRAP   : c_int = 5;       /* Trace trap (POSIX).  */
    pub static  SIGABRT   : c_int = 6;       /* Abort (ANSI).  */
    pub static  SIGIOT    : c_int = 6;       /* IOT trap (4.2 BSD).  */
    pub static  SIGBUS    : c_int = 7;       /* BUS error (4.2 BSD).  */
    pub static  SIGFPE    : c_int = 8;       /* Floating-point exception (ANSI).  */
    pub static  SIGKILL   : c_int = 9;       /* Kill, unblockable (POSIX).  */
    pub static  SIGUSR1   : c_int = 10;      /* User-defined signal 1 (POSIX).  */
    pub static  SIGSEGV   : c_int = 11;      /* Segmentation violation (ANSI).  */
    pub static  SIGUSR2   : c_int = 12;      /* User-defined signal 2 (POSIX).  */
    pub static  SIGPIPE   : c_int = 13;      /* Broken pipe (POSIX).  */
    pub static  SIGALRM   : c_int = 14;      /* Alarm clock (POSIX).  */
    pub static  SIGTERM   : c_int = 15;      /* Termination (ANSI).  */
    pub static  SIGSTKFLT : c_int = 16;      /* Stack fault.  */
    pub static  SIGCLD    : c_int = SIGCHLD; /* Same as SIGCHLD (System V).  */
    pub static  SIGCHLD   : c_int = 17;      /* Child status has changed (POSIX).  */
    pub static  SIGCONT   : c_int = 18;      /* Continue (POSIX).  */
    pub static  SIGSTOP   : c_int = 19;      /* Stop, unblockable (POSIX).  */
    pub static  SIGTSTP   : c_int = 20;      /* Keyboard stop (POSIX).  */
    pub static  SIGTTIN   : c_int = 21;      /* Background read from tty (POSIX).  */
    pub static  SIGTTOU   : c_int = 22;      /* Background write to tty (POSIX).  */
    pub static  SIGURG    : c_int = 23;      /* Urgent condition on socket (4.2 BSD).  */
    pub static  SIGXCPU   : c_int = 24;      /* CPU limit exceeded (4.2 BSD).  */
    pub static  SIGXFSZ   : c_int = 25;      /* File size limit exceeded (4.2 BSD).  */
    pub static  SIGVTALRM : c_int = 26;      /* Virtual alarm clock (4.2 BSD).  */
    pub static  SIGPROF   : c_int = 27;      /* Profiling alarm clock (4.2 BSD).  */
    pub static  SIGWINCH  : c_int = 28;      /* Window size change (4.3 BSD, Sun).  */
    pub static  SIGPOLL   : c_int = SIGIO;   /* Pollable event occurred (System V).  */
    pub static  SIGIO     : c_int = 29;      /* I/O now possible (4.2 BSD).  */
    pub static  SIGPWR    : c_int = 30;      /* Power failure restart (System V).  */
    pub static  SIGSYS    : c_int = 31;      /* Bad system call.  */
    pub static  SIGUNUSED : c_int = 31;

    pub static  _NSIG     : c_int = 65;      /* Biggest signal number + 1 
                                                (including real-time signals).  */


