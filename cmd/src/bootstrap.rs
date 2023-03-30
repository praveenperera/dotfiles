use eyre::Result;
use xshell::{cmd, Shell};

pub fn run(sh: &Shell) -> Result<()> {
    println!("bootstrap");
    println!("{}", std::env::consts::OS);
    Ok(())
}
