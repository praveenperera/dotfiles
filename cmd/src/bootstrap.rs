use eyre::Result;
use sailfish::TemplateOnce;
use xshell::{cmd, Shell};

use crate::os::Os;

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

    if Os::current() == Os::MacOS {
        let osx_defaults = OsxDefaults {}.render_once()?;
        create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;
    }

    log::info!("writing zshrc to {}", path.display());
    sh.write_file(&path, zshrc.render_once()?)?;

    Ok(())
}

fn create_and_run_file(sh: &Shell, contents: &str, file: &str) -> Result<()> {
    let tmp_dir = sh.create_temp_dir()?;
    let tmp_path = tmp_dir.path().join(file);
    sh.write_file(&tmp_path, contents)?;
    cmd!(sh, "zsh {tmp_path}").run()?;

    Ok(())
}
