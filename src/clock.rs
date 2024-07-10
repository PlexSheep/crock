#![warn(clippy::pedantic, clippy::style, clippy::nursery)]
#![allow(clippy::question_mark_used)]

use chrono::{DateTime, Datelike, Local, SubsecRound, Timelike};
use clap::Parser;
use libpt::cli::args::HELP_TEMPLATE;
use libpt::cli::clap::ArgGroup;
use libpt::cli::{args::VerbosityLevel, clap};
use libpt::log::{debug, trace};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, poll, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, LineGauge, Padding, Paragraph};
use ratatui::Terminal;
use std::io::Stdout;
use std::thread::{sleep, sleep_ms};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBarLength {
    Minute,
    Hour,
    Custom(i64),
    Day,
}

impl TimeBarLength {
    pub(crate) const fn as_secs(self) -> i64 {
        match self {
            Self::Minute => 60,
            Self::Day => 24 * 60 * 60,
            Self::Hour => 60 * 60,
            Self::Custom(secs) => secs,
        }
    }
}

impl Default for TimeBarLength {
    fn default() -> Self {
        Self::Minute
    }
}

/// Make your terminal into a big clock
#[derive(Parser, Debug, Clone)]
#[command(help_template = HELP_TEMPLATE, author, version)]
#[clap(group( ArgGroup::new("timebarlen") .args(&["minute","day", "hour", "custom"]),))]
pub struct Clock {
    #[command(flatten)]
    pub verbose: VerbosityLevel,
    /// Show time since start
    #[clap(short, long)]
    pub timer: bool,

    // timebar options
    #[clap(short, long)]
    pub minute: bool,
    #[clap(short, long)]
    pub day: bool,
    #[clap(short = 'o', long)]
    pub hour: bool,
    #[clap(short, long)]
    pub custom: Option<i64>,
    #[clap(skip)]
    last_reset: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiData {
    fdate: [String; 2],
    ftime: [String; 2],
    timebar_ratio: [Option<f64>; 2],

    data_idx: usize,
}

impl UiData {
    pub fn update(&mut self, fdate: String, ftime: String, timebar_ratio: Option<f64>) {
        self.data_idx ^= 1;
        self.fdate[self.data_idx] = fdate;
        self.ftime[self.data_idx] = ftime;
        self.timebar_ratio[self.data_idx] = timebar_ratio;
        trace!(
            "data after update: {:#?}\nconsidered changed: {}",
            self,
            self.changed()
        );
    }

    /// did the data change with the last update?
    #[must_use]
    #[inline]
    pub fn changed(&self) -> bool {
        let r = 
            self.fdate[0] != self.fdate[1] || self.ftime[0] != self.ftime[1]
            //&& self.timebar_ratio[0] == self.timebar_ratio[1]
            //  NOTE: the timebar ratio is discarded, so that we only render the ui when the time (second) changes
        ;
        trace!("changed: {r}");
        r
    }

    #[must_use]
    #[inline]
    pub(crate) fn fdate(&self) -> &str {
        &self.fdate[self.data_idx]
    }

    #[must_use]
    #[inline]
    pub(crate) fn ftime(&self) -> &str {
        &self.ftime[self.data_idx]
    }

    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // no it's not const
    pub(crate) fn timebar_ratio(&self) -> Option<f64> {
        self.timebar_ratio[self.data_idx]
    }
}

impl Clock {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    fn timebar_len(&self) -> Option<TimeBarLength> {
        if self.minute {
            Some(TimeBarLength::Minute)
        } else if self.day {
            Some(TimeBarLength::Day)
        } else if self.hour {
            Some(TimeBarLength::Hour)
        } else {
            // this feels weird but is the same
            self.custom.map(TimeBarLength::Custom)
        }
    }

    // FIXME: This generally works, but we want 0% at the start and 100% at the end, which does not 
    // fully work.
    // We also want 50% at the half etc
    fn timebar_ratio(&self) -> Option<f64> {
        let len = self.timebar_len()?;
        let since = (Local::now()
            .signed_duration_since(self.last_reset.unwrap())
            .num_seconds()+1) as f64;
        Some((since / len.as_secs() as f64).min(1.0).max(0.0))
    }

    fn maybe_reset_since_zero(&mut self) {
        if let Some(len) = self.timebar_len() {
            trace!("Local Time: {}", Local::now());
            // BUG: these resets trigger multiple times
            let since_last_reset = Local::now().signed_duration_since(self.last_reset.unwrap());
            match len {
                TimeBarLength::Custom(_) => {
                    if since_last_reset.num_seconds() >= 1
                        && since_last_reset.num_seconds() >= len.as_secs()
                    {
                        self.last_reset = Some(Local::now());
                    }
                }
                TimeBarLength::Minute => {
                    if since_last_reset.num_seconds() >= 1 && Local::now().second() == 0 {
                        self.last_reset = Some(Local::now());
                        debug!("reset the time of the time bar (minute)");
                    }
                }
                TimeBarLength::Hour => {
                    if since_last_reset.num_minutes() >= 1 && Local::now().minute() == 0 {
                        self.last_reset = Some(Local::now());
                        debug!("reset the time of the time bar (hour)");
                    }
                }
                TimeBarLength::Day => {
                    if since_last_reset.num_hours() >= 1 && Local::now().hour() == 0 {
                        self.last_reset = Some(Local::now());
                        debug!("reset the time of the time bar (day)");
                    }
                }
            }
        }
    }

    fn setup_last_reset(&mut self) {
        if let Some(len) = self.timebar_len() {
            trace!("Local Time: {}", Local::now());
            match len {
                TimeBarLength::Custom(_) => {
                    self.last_reset = Some(Local::now());
                }
                TimeBarLength::Minute => {
                    self.last_reset = Some(
                        Local::now()
                            .with_second(0)
                            .expect("tried to use a time that does not exist"),
                    );
                }
                TimeBarLength::Hour => {
                    self.last_reset = Some(
                        Local::now()
                            .with_minute(0)
                            .expect("tried to use a time that does not exist"),
                    );
                }
                TimeBarLength::Day => {
                    self.last_reset = Some(
                        Local::now()
                            .with_hour(0)
                            .expect("tried to use a time that does not exist"),
                    );
                }
            }
            debug!("set up initial last reset as {:#?}", self.last_reset);
        }
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        self.setup_last_reset();
        Ok(())
    }

    pub(crate) fn run(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        let mut uidata: UiData = UiData::default();
        self.setup()?;
        loop {
            let raw_time = chrono::Local::now().round_subsecs(0);
            let splits: Vec<String> = raw_time
                .naive_local()
                .to_string()
                .split_whitespace()
                .map(str::to_string)
                .collect();

            uidata.update(splits[0].clone(), splits[1].clone(), self.timebar_ratio());
            if uidata.changed() {
                self.ui(terminal, &uidata)?;
            }
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q')
                        || key.code == KeyCode::Esc
                        || (key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c'))
                    {
                        return Ok(());
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
    }
    fn on_tick(&mut self) {
        self.maybe_reset_since_zero();
    }
    fn ui(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        data: &UiData,
    ) -> anyhow::Result<()> {
        let clockw = tui_big_text::BigText::builder()
            .style(Style::new().red())
            .lines(vec![data.ftime().into()])
            .alignment(Alignment::Center)
            .build()
            .expect("could not render time widget");
        terminal.draw(|frame| {
            debug!("rendering the ui");
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
            frame.render_widget(space, root);
            let parts = Self::partition(a);

            // render the timebar which counts up to the full minute and so on
            //
            // Will not be rendered if it is None
            let timebarw: Option<LineGauge> = if self.timebar_len().is_some() {
                debug!("time bar ration: {:?}", data.timebar_ratio());
                let tmp = LineGauge::default()
                    .filled_style(Style::default().blue())
                    .unfilled_style(Style::default())
                    .block(Block::new().padding(Padding::new(
                        parts[2].left() / 10,
                        parts[2].right() / 6,
                        0,
                        0,
                    )))
                    .ratio(data.timebar_ratio().unwrap());
                Some(tmp)
            } else {
                None
            };

            // render the small date
            let datew = Paragraph::new(data.fdate())
                .blue()
                .alignment(Alignment::Left)
                .block(Block::new().padding(Padding::new(
                    parts[1].left(),
                    parts[1].right() / 3,
                    0,
                    0,
                )));
            frame.render_widget(&timebarw, parts[2]);
            frame.render_widget(datew, parts[1]);
            // render the clock
            frame.render_widget(clockw, parts[0]);
        })?;
        debug!("done rendering the ui");
        Ok(())
    }
    fn partition(r: Rect) -> Vec<Rect> {
        let part = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(r);
        let subparts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Ratio(1, 2)])
            .split(part[1]);

        vec![part[0], subparts[0], subparts[1]]
    }
}
