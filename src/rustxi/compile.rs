use std::path;
use syntax::diagnostic;
use rustc::driver::{driver, session};
use rustc::back::link::jit;

/**
 *  here is where the heart of the jit-repl will be: here
 *   we actually compile and run the code.
 **/
pub fn compile_and_run(code: &str) {
    let options = @session::options {
        crate_type: session::lib_crate,
        binary: super::PROGRAM_NAME.to_managed(),
        addl_lib_search_paths: @mut ~[path::Path("/home/minh/opt/lib")],
        jit: true,
        .. (*session::basic_options()).clone()
    };
    // the link directive is to silence rustc's warning
    // no_mangle is to preserve the name so that rustc::back::link::exec can
    // extract it.
    let input = driver::str_input(format!(r###"
\#[link(name="rustxi_lib",
        vers="0.0")];

\#[no_mangle]
fn my_fn() -> int \{
    {:s}
\}"###, code).to_managed());
    let sess = driver::build_session(options, @diagnostic::DefaultEmitter as
                                        @diagnostic::Emitter);
    let cfg = driver::build_configuration(sess);
    let outputs = driver::build_output_filenames(&input, &None, &None, [], sess);

    let crate = driver::phase_1_parse_input(sess, cfg.clone(), &input);
    let expanded_crate = driver::phase_2_configure_and_expand(sess, cfg, crate);
    let analysis = driver::phase_3_run_analysis_passes(sess, &expanded_crate);
    println!("[compile_and_run] cannot get here");
    let trans = driver::phase_4_translate_to_llvm(sess, expanded_crate, &analysis, outputs);
    driver::phase_5_run_llvm_passes(sess, &trans, outputs);

    jit::consume_engine();
}
