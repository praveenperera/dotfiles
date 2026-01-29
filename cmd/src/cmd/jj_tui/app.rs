use super::tree::TreeState;
use super::ui;
use crate::jj_lib_helpers::JjRepo;
use eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use xshell::Shell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Help,
}

pub struct App {
    pub tree: TreeState,
    pub mode: Mode,
    pub should_quit: bool,
    #[allow(dead_code)]
    sh: Shell,
}

impl App {
    pub fn new(sh: &Shell) -> Result<Self> {
        let jj_repo = JjRepo::load(None)?;
        let tree = TreeState::load(&jj_repo)?;

        Ok(Self {
            tree,
            mode: Mode::Normal,
            should_quit: false,
            sh: sh.clone(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        let result = self.run_loop(&mut terminal);
        ratatui::restore();
        result
    }

    fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let viewport_height = terminal.size()?.height.saturating_sub(3) as usize;
            self.tree.update_scroll(viewport_height);

            terminal.draw(|frame| ui::render(frame, self))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match self.mode {
            Mode::Normal => self.handle_normal_key(code),
            Mode::Help => self.handle_help_key(code),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.mode = Mode::Help,

            KeyCode::Char('j') | KeyCode::Down => self.tree.move_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => self.tree.move_cursor_up(),
            KeyCode::Char('g') => self.tree.move_cursor_top(),
            KeyCode::Char('G') => self.tree.move_cursor_bottom(),
            KeyCode::Char('@') => self.tree.jump_to_working_copy(),

            KeyCode::Char('f') => self.tree.toggle_full_mode(),

            _ => {}
        }
    }

    fn handle_help_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}
