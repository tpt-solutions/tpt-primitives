//! Generates JSON Schema and CDDL definitions for the canonical types of
//! `tpt-primitives` into `schemas/`. CI runs this with `--check` and fails
//! if the checked-in schemas have drifted from the Rust types.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check = args.iter().any(|a| a == "--check");
    if check {
        println!("schema drift check not implemented yet");
        std::process::exit(2);
    }
    println!("schema generation not implemented yet");
    std::process::exit(2);
}
