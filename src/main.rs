use std::process;
fn main() {
    match rust_ping::run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}
