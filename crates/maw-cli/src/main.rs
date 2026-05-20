fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let output = maw_cli::run_cli(&argv);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    std::process::exit(output.code);
}
