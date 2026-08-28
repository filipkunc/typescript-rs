mod lsp;

use std::{env, ffi::OsString, fs, path::Path, process::ExitCode};

use typescript_rs::check_source;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let files: Vec<_> = arguments.collect();

    if files.first().is_some_and(|argument| argument == "--lsp") {
        return run_lsp(&files[1..]);
    }

    if files.is_empty()
        || files
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print_usage();
        return if files.is_empty() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    let mut had_error = false;
    for file in files {
        let path = Path::new(&file);
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                had_error = true;
                continue;
            }
        };
        let file_name = path.to_string_lossy();
        let result = check_source(&file_name, &source);
        if !result.is_ok() {
            had_error = true;
            for line in result.render_concise(&source).lines() {
                eprintln!("{}:{line}", path.display());
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_lsp(arguments: &[OsString]) -> ExitCode {
    if arguments.iter().any(|argument| argument != "--stdio") {
        print_usage();
        return ExitCode::from(2);
    }

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to start the LSP runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(lsp::serve_stdio());
    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!("Usage: tsrs <file.ts> [file.ts ...]\n       tsrs --lsp [--stdio]");
}
