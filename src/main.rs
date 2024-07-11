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
            .display_time(true)
            .build()?;
    } else {
        // no logger
    }
    debug!("set up logger");

    #[cfg(debug_assertions)]
    mock_tests();

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

#[cfg(debug_assertions)]
#[allow(clippy::cast_precision_loss)]
fn mock_tests() {
    use chrono::{Local, Timelike};
    use libpt::log::info;

    use self::clock::UiData;
    info!("doing the mock tests");
    {
        let mut c = Clock::parse_from(["some exec", "-mvvv"]);
        let now = Local::now();
        c.last_reset = Some(now.with_second(0).unwrap());

        assert_eq!(c.timebar_ratio(now.with_second(30).unwrap()), Some(0.5));
        info!("30s=0.5");
        assert_eq!(
            c.timebar_ratio(now.with_second(59).unwrap()),
            Some(0.9833333333333333)
        );
        info!("60s=1.0");
        assert_eq!(c.timebar_ratio(now.with_second(0).unwrap()), Some(0.0));
        info!("0s=0.0");
    }
    {
        let mut data = UiData::default();
        data.update("date".to_owned(), "time".to_owned(), Some(0.1));
        assert_eq!(data.timebar_ratio(), Some(0.1));
        data.update("date".to_owned(), "time".to_owned(), Some(0.2));
        assert_eq!(data.timebar_ratio(), Some(0.2));
        data.update("date".to_owned(), "time".to_owned(), Some(0.3));
        assert_eq!(data.timebar_ratio(), Some(0.3));
    }
    info!("finished the mock tests");
}
