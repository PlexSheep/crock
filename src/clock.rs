#![warn(clippy::pedantic, clippy::style, clippy::nursery)]
#![allow(clippy::question_mark_used)]
use clap::Parser;
use libpt::cli::{args::VerbosityLevel, args::HELP_TEMPLATE, clap};

use chrono::SubsecRound;
use ratatui::crossterm::event::{
    self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use ratatui::layout::Alignment;
use ratatui::widgets::{Block, Padding};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::Paragraph,
    Terminal,
};
use std::io::Stdout;
use std::time::Duration;

/// Make your terminal into a big clock
#[derive(Parser, Debug, Clone, PartialEq, Eq, Hash)]
#[command(help_template = HELP_TEMPLATE)]
pub(crate) struct Clock {
    #[command(flatten)]
    pub verbose: VerbosityLevel,
}

impl Clock {
    pub(crate) fn run(
        self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        loop {
            let raw_time = chrono::Local::now().round_subsecs(0);
            let splits: Vec<String> = raw_time
                .naive_local()
                .to_string()
                .split_whitespace()
                .map(str::to_string)
                .collect();
            let fdate: String = splits[0].clone();
            let ftime: String = splits[1].clone();
            terminal.draw(|frame| {
                let root = frame.size();
                let space = Block::bordered()
                    .padding(Padding::new(
                        root.width / 8,
                        root.width / 8,
                        root.height / 8,
                        root.height / 8,
                    ))
                    .title(env!("CARGO_PKG_NAME"))
                    .title_bottom(env!("CARGO_PKG_VERSION"))
                    .title_alignment(Alignment::Center)
                    .title_style(Style::new().bold());
                let a = space.inner(root);
                let parts = Self::partition(a);
                let timew = tui_big_text::BigText::builder()
                    .style(Style::new().red())
                    .lines(vec![ftime.into()])
                    .alignment(Alignment::Center)
                    .build()
                    .expect("could not render time widget");
                let datew = Paragraph::new(fdate)
                    .blue()
                    .alignment(Alignment::Left)
                    .block(Block::new().padding(Padding::new(
                        parts.0.left(),
                        parts.0.right() / 3,
                        0,
                        0,
                    )));
                frame.render_widget(space, root);
                frame.render_widget(timew, parts.1);
                frame.render_widget(datew, parts.0);
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
        Ok(())
    }
    fn partition(r: Rect) -> (Rect, Rect) {
        let part = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(13), Constraint::Min(0)])
            .split(r);

        (part[0], part[1])
    }
}
