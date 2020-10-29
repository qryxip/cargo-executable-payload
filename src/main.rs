use cargo_b64encode::{Opt, Shell};
use structopt::{clap, StructOpt as _};

fn main() {
    let opt = Opt::from_args();
    let mut shell = Shell::new();
    if let Err(err) = cargo_b64encode::run(opt, &mut shell) {
        exit_with_error(err, &mut shell);
    }
}

fn exit_with_error(err: anyhow::Error, shell: &mut Shell) -> ! {
    if let Some(err) = err.downcast_ref::<clap::Error>() {
        err.exit();
    }
    let _ = shell.error(&err);
    for cause in err.chain().skip(1) {
        let _ = writeln!(shell.err(), "\nCaused by:");
        for line in cause.to_string().lines() {
            let _ = match line {
                "" => writeln!(shell.err()),
                line => writeln!(shell.err(), "  {}", line),
            };
        }
    }
    std::process::exit(1);
}
