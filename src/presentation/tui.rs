use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
};
use std::io;

pub fn run_app() -> Result<()>
{
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop
    {
        terminal.draw(|f| {
            let size = f.area();
            let block = Block::default().title(" Ananicy Manager TUI ").borders(Borders::ALL);
            let p = Paragraph::new("Welcome to the Manager!\n\nPress 'q' to quit.").block(block);

            f.render_widget(p, size);
        })?;

        if event::poll(std::time::Duration::from_millis(250))?
        {
            if let Event::Key(key) = event::read()?
            {
                if key.code == KeyCode::Char('q')
                {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
