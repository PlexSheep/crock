use std::io;

use libpt::cli::clap::Parser;
use libpt::log::{debug, Level, Logger};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use self::clock::Clock;

mod clock;

fn main() -> anyhow::Result<()> {
    // setup the cli
    let clock = Clock::parse();
    if clock.verbose.level() >= Level::DEBUG {
        let _logger = Logger::builder()
            .log_to_file(true)
            .log_dir("/tmp/crock/".into())
            .set_level(clock.verbose.level())
            .display_time(false)
            .build()?;
    } else {
        // no logger
    }
    debug!("set up logger");

    debug!("taking over terminal");
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    debug!("entering clock");
    let result = clock.run(&mut terminal);

    debug!("restoring terminal");
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    debug!("done");
    result
}
