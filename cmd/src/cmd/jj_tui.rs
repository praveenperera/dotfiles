mod app;
mod keybindings;
mod tree;
mod ui;

use eyre::Result;
use xshell::Shell;

pub fn run(sh: &Shell) -> Result<()> {
    let mut app = app::App::new(sh)?;
    app.run()
}
