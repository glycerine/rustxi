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
        crate_type: session::unknown_crate,
        binary: super::PROGRAM_NAME.to_managed(),
        addl_lib_search_paths: @mut ~[path::Path("/home/minh/Documents/Workplace/rust/rust/x86_64-unknown-linux-gnu/stage2/lib")],
        jit: true,
        .. (*session::basic_options()).clone()
    };
    let input = driver::str_input("fn main() {\n\tprint(\"Hello, world\");\n}".to_managed());
    let sess = driver::build_session(options, @diagnostic::DefaultEmitter as
                                        @diagnostic::Emitter);
    let cfg = driver::build_configuration(sess);
    let outputs = driver::build_output_filenames(&input, &None, &None, [], sess);

    let crate = driver::phase_1_parse_input(sess, cfg.clone(), &input);
    let expanded_crate = driver::phase_2_configure_and_expand(sess, cfg, crate);
    let analysis = driver::phase_3_run_analysis_passes(sess, expanded_crate);
    let trans = driver::phase_4_translate_to_llvm(sess, expanded_crate, &analysis, outputs);
    driver::phase_5_run_llvm_passes(sess, &trans, outputs);

    jit::consume_engine();

    println("Got here");
}
