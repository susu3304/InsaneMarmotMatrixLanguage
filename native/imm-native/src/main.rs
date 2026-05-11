const VERSION: &str = "0.1.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("insane marmot matrix native {}", VERSION);
        return;
    }

    eprintln!(
        "imm-native is a parity-gated preview scaffold. Run the Python reference with `imm law` before enabling native commands."
    );
    std::process::exit(2);
}
