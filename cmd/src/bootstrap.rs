use eyre::Result;
use sailfish::TemplateOnce;
use xshell::{cmd, Shell};

use crate::{command_exists, os::Os};
use colored::Colorize;

#[derive(TemplateOnce)]
#[template(path = "zshrc.stpl")]
struct Zshrc {
    os: Os,
}

#[derive(TemplateOnce)]
#[template(path = "osx_defaults.zsh.stpl")]
struct OsxDefaults {}

pub fn run(sh: &Shell) -> Result<()> {
    let path = crate::dotfiles_dir().join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    if Os::current() == Os::MacOS {}

    println!("writing zshrc to {}", path.display().to_string().green());
    sh.write_file(&path, zshrc.render_once()?)?;

    match Os::current() {
        Os::Linux => {
            cmd!(sh, "sudo apt-get install -y zsh").run()?;
            cmd!(sh, "sudo apt-get install -y fzf").run()?;
        }
        Os::MacOS => {
            let osx_defaults = OsxDefaults {}.render_once()?;
            create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;

            install_brew_and_tools(sh)?;
        }
    }

    Ok(())
}

fn install_brew_and_tools(sh: &Shell) -> Result<()> {
    if !command_exists(sh, "brew") {
        println!("{} {}", "brew not found".red(), "installing...".green());

        cmd!(sh, "/bin/bash -c '$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)'")
        .run()?;
    }

    Ok(())
}

fn create_and_run_file(sh: &Shell, contents: &str, file: &str) -> Result<()> {
    let tmp_dir = sh.create_temp_dir()?;
    let tmp_path = tmp_dir.path().join(file);
    sh.write_file(&tmp_path, contents)?;

    println!("running {}", file.green());
    cmd!(sh, "zsh {tmp_path}").quiet().run()?;

    Ok(())
}
