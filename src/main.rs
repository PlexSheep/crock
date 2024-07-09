use chrono::SubsecRound;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::poll,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    style::Stylize,
};
use std::{io, time::Duration};

const TITLE: &str = "Crock";

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        let raw_time = chrono::Utc::now().round_subsecs(0);
        let splits: Vec<String> = raw_time
            .naive_local()
            .to_string()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let fdate: String = splits[0].clone();
        let ftime: String = splits[1].clone();
        terminal.draw(|f| {
            let timew = tui_big_text::BigText::builder()
                .style(Style::new().red())
                .lines(vec![ftime.into()])
                .build()
                .expect("could not render time widget");
            let datew = Paragraph::new(fdate).blue();
            f.render_widget(datew, centered_rect(f.size(), 45, 80));
            f.render_widget(timew, centered_rect(f.size(), 45, 60));
        })?;
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q')
                    || key.code == KeyCode::Esc
                    || (key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c'))
                {
                    break;
                }
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.size(), 50, 50);
/// ```
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
