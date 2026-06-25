mod cli;

#[cfg(windows)]
const WINDOWS_CLI_STACK_SIZE: usize = 8 * 1024 * 1024;

#[cfg(windows)]
fn main() {
    let handle = std::thread::Builder::new()
        .name("skillspec-cli".to_owned())
        .stack_size(WINDOWS_CLI_STACK_SIZE)
        .spawn(main_inner)
        .expect("failed to start skillspec CLI thread");

    if let Err(payload) = handle.join() {
        std::panic::resume_unwind(payload);
    }
}

#[cfg(not(windows))]
fn main() {
    main_inner();
}

fn main_inner() {
    if let Err(error) = cli::run() {
        skillspec::report::error(error);
        std::process::exit(1);
    }
}
