use std::{env, fs, path::Path, process::ExitCode};

use typescript_rs::check_source;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let files: Vec<_> = arguments.collect();

    if files.is_empty()
        || files
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        eprintln!("Usage: tsrs <file.ts> [file.ts ...]");
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
