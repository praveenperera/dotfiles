use eyre::Result;
use sailfish::TemplateOnce;
use xshell::Shell;

use crate::os::Os;

#[derive(TemplateOnce)]
#[template(path = "zshrc.stpl")]
struct Zshrc {
    os: Os,
}

pub fn run(sh: &Shell) -> Result<()> {
    let path = crate::dotfiles_dir().join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    log::info!("writing zshrc to {}", path.display());
    sh.write_file(&path, zshrc.render_once()?)?;

    Ok(())
}
