#![warn(clippy::pedantic, clippy::style, clippy::nursery)]
// That's useful and sometimes I don't need extra stuff
#![allow(clippy::question_mark_used)]
// allows us to cast u64 to i64, if the number is too high
// enough time has passed that I don't care
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)] // it should only be positive anyway

use chrono::{DateTime, Local, SubsecRound, Timelike};
use clap::Parser;
use libpt::cli::args::HELP_TEMPLATE;
use libpt::cli::clap::ArgGroup;
use libpt::cli::{args::VerbosityLevel, clap};
use libpt::log::{debug, error, trace};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, poll, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Terminal;
use std::collections::HashMap;
use std::io::{Cursor, Stdout, Write};
use std::time::Instant;

pub mod timebar;
pub mod ui;
use timebar::TimeBarLength;
use ui::Data;

/// Make your terminal into a big clock
#[derive(Parser, Debug, Clone)]
#[command(help_template = HELP_TEMPLATE, author, version)]
#[clap(group( ArgGroup::new("timebarlen") .args(&["minute","day", "hour", "custom", "countdown", "timer"]),))]
#[allow(clippy::struct_excessive_bools)] // the struct is for cli parsing and we already use an
                                         // ArgGroup
pub struct Clock {
    #[command(flatten)]
    pub verbose: VerbosityLevel,
    /// Show time since start
    #[clap(short, long)]
    pub timer: bool,

    // timebar options
    /// show a time bar that tracks progress of the minute
    #[clap(short, long)]
    pub minute: bool,
    /// show a time bar that tracks progress of the day
    #[clap(short, long)]
    pub day: bool,
    /// show a time bar that tracks progress of the hour
    #[clap(short = 'o', long)]
    pub hour: bool,
    /// show a time bar that tracks progress of a custom duration
    ///
    /// Precision: only to seconds
    #[clap(short, long, value_parser = humantime::parse_duration)]
    pub custom: Option<std::time::Duration>,
    /// show a time bar that tracks progress of a custom duration without resetting
    ///
    /// Precision: only to seconds
    #[clap(short = 'u', long, value_parser = humantime::parse_duration)]
    pub countdown: Option<std::time::Duration>,
    /// Play a notification sound when the countdown is up
    #[cfg(feature = "sound")]
    #[clap(short, long, default_value_t = true)]
    pub sound: bool,

    // internal variables
    #[clap(skip)]
    pub(crate) last_reset: Option<DateTime<Local>>,
    #[clap(skip)]
    pub(crate) did_notify: bool,
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
        } else if self.timer {
            Some(TimeBarLength::Timer)
        } else if self.countdown.is_some() {
            Some(TimeBarLength::Countup(
                self.countdown.unwrap().as_secs() as i64
            ))
        } else if self.custom.is_some() {
            Some(TimeBarLength::Custom(self.custom.unwrap().as_secs() as i64))
        } else {
            None
        }
    }

    #[allow(clippy::cast_precision_loss)] // okay, good to know, but I accept the loss. It
                                          // shouldn't come to more than 2^52 seconds anyway
    pub(crate) fn timebar_ratio(&self, current_time: DateTime<Local>) -> Option<f64> {
        let len = self.timebar_len()?;
        let since = current_time
            .signed_duration_since(self.last_reset.unwrap())
            .num_seconds() as f64;
        #[cfg(debug_assertions)]
        if since < 1.0 {
            trace!("ratio calculation since is now <1: {:#?}", since);
        }
        Some((since / len.as_secs() as f64).clamp(0.0, 1.0))
    }

    pub(crate) fn maybe_reset_since_zero(&mut self) {
        if let Some(len) = self.timebar_len() {
            let since_last_reset = Local::now().signed_duration_since(self.last_reset.unwrap());
            match len {
                TimeBarLength::Countup(_) | TimeBarLength::Timer => {
                    // the count up should not reset. If the time is over, just keep it at 100%
                }
                TimeBarLength::Custom(_) => {
                    // BUG: this is not consistent, sometimes leads to wrong seconds
                    if since_last_reset.num_milliseconds() >= 100
                        && since_last_reset.num_seconds() >= len.as_secs()
                    {
                        self.last_reset = Some(Local::now().round_subsecs(0));
                    }
                }
                TimeBarLength::Minute => {
                    if since_last_reset.num_seconds() >= 1 && Local::now().second() == 0 {
                        self.last_reset = Some(
                            Local::now()
                                .round_subsecs(0)
                                .with_second(1)
                                .expect("tried to use a time that does not exist"),
                        );
                        debug!("reset the time of the time bar (minute)");
                    }
                }
                TimeBarLength::Hour => {
                    if since_last_reset.num_minutes() >= 1 && Local::now().minute() == 0 {
                        self.last_reset = Some(
                            Local::now()
                                .round_subsecs(0)
                                .with_second(1)
                                .expect("tried to use a time that does not exist"),
                        );
                        debug!("reset the time of the time bar (hour)");
                    }
                }
                TimeBarLength::Day => {
                    if since_last_reset.num_hours() >= 1 && Local::now().hour() == 0 {
                        self.last_reset = Some(
                            Local::now()
                                .round_subsecs(0)
                                .with_second(1)
                                .expect("tried to use a time that does not exist"),
                        );
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
                TimeBarLength::Custom(_) | TimeBarLength::Countup(_) | TimeBarLength::Timer => {
                    self.last_reset = Some(Local::now());
                }
                TimeBarLength::Minute => {
                    self.last_reset = Some(
                        Local::now()
                            .round_subsecs(0)
                            .with_second(1)
                            .expect("tried to use a time that does not exist"),
                    );
                }
                TimeBarLength::Hour => {
                    self.last_reset = Some(
                        Local::now()
                            .round_subsecs(0)
                            .with_second(1)
                            .expect("tried to use a time that does not exist")
                            .with_minute(0)
                            .expect("tried to use a time that does not exist"),
                    );
                }
                TimeBarLength::Day => {
                    self.last_reset = Some(
                        Local::now()
                            .round_subsecs(0)
                            .with_second(1)
                            .expect("tried to use a time that does not exist")
                            .with_minute(0)
                            .expect("tried to use a time that does not exist")
                            .with_hour(0)
                            .expect("tried to use a time that does not exist"),
                    );
                }
            }
            debug!("set up initial last reset as {:#?}", self.last_reset);
        }
    }

    #[allow(clippy::unnecessary_wraps)] // we have that to be future proof
    pub(crate) fn setup(&mut self) -> anyhow::Result<()> {
        self.setup_last_reset();
        Ok(())
    }

    /// Run the clock TUI
    ///
    /// # Errors
    ///
    /// * The [setup](Self::setup) fails
    /// * Drawing the [ui](Self::ui) fails
    /// * Polling or reading an event fails
    pub(crate) fn run(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        let tick_rate = std::time::Duration::from_millis(100);
        let mut last_tick = Instant::now();
        let mut uidata: Data = Data::new(self.timebar_len());
        self.setup()?;
        loop {
            let raw_time = chrono::Local::now().round_subsecs(0);
            let splits: Vec<String> = raw_time
                .naive_local()
                .to_string()
                .split_whitespace()
                .map(str::to_string)
                .collect();

            // We somehow fill timebar_ratio with a bad value here if we don't add 1 second. It's
            // always the value that would be right for now-1s. The start of the minute is
            // special, with this strategy it is 100%. #10
            //
            // If we manually add a second, it works as expected, but it feels weird. We use the
            // same time for all of the datapoints here, so it can't be because of time diff in
            // calculation. I noticed that we don't start at 0% this way (with len=minute)
            // . Normally, chrono does not include 60 seconds, only letting it range between 0 and
            // 59. This makes sense but feels weird to the human understanding, of course there are
            // seconds in a minute! If we do it this way, we don't quite start at 0%, but 100%,
            // which feels correct.
            //
            // In short: if we add a second here, we get the correct percentages. 01:00 is 100%,
            // 01:30 is 50%, 01:59 is 98%, 01:60 does not exist because that's how counting from
            // 0 works.

            let now = raw_time + chrono::Duration::seconds(1);
            uidata.update(
                now,
                splits[0].clone(),
                splits[1].clone(),
                self.timebar_ratio(now),
            );
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
    #[allow(clippy::cast_possible_truncation)] // if we have that much padding, please truncate
    fn ui(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        data: &Data,
    ) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            debug!("rendering the ui");
            let root = frame.size();
            let space = Block::bordered()
                .padding(Padding::new(
                    root.width / 16,
                    root.width / 16,
                    root.height / 16,
                    root.height / 16,
                ))
                .title(env!("CARGO_PKG_NAME"))
                .title_bottom(env!("CARGO_PKG_VERSION"))
                .title_alignment(Alignment::Center)
                .title_style(Style::new().bold());
            let inner_rect = space.inner(root);
            frame.render_widget(space, root);
            let parts = Self::partition(inner_rect);

            let mut clockw = tui_big_text::BigText::builder();
            if inner_rect.width > 80 {
                clockw.pixel_size(tui_big_text::PixelSize::Full);
            } else {
                clockw.pixel_size(tui_big_text::PixelSize::Quadrant);
            }

            let clockw = clockw
                .style(Style::new().red())
                .lines(vec![data.ftime().into()])
                .alignment(Alignment::Center)
                .build()
                .expect("could not render time widget");

            // render the timebar which counts up to the full minute and so on
            //
            // Will not be rendered if it is None
            let timebarw_padding = [
                (f32::from(parts["timebarw"].width) * 0.43) as u16,
                (f32::from(parts["timebarw"].width) * 0.25) as u16,
            ];
            let timebarw = ui::timebarw(self, data, &timebarw_padding, inner_rect);
            let timebarw_label: Option<Paragraph> =
                ui::timebarw_label(self, data, &timebarw_padding, inner_rect);

            // render the small date
            let datew = Paragraph::new(data.fdate())
                .blue()
                .block(Block::default().padding(Padding::right(2)))
                .alignment(Alignment::Right);
            frame.render_widget(&timebarw, parts["timebarw"]);
            frame.render_widget(&timebarw_label, parts["timebarw_label"]);
            frame.render_widget(datew, parts["datew"]);
            // render the clock
            frame.render_widget(clockw, parts["clockw"]);
        })?;
        debug!("done rendering the ui");
        Ok(())
    }
    fn notify(&mut self) -> anyhow::Result<()> {
        Self::beep()?;
        #[cfg(feature = "sound")]
        if self.sound {
            std::thread::spawn(|| {
                use rodio::{Decoder, OutputStream, Sink};
                // only 30 KiB, so let's just include it in the binary and not worry about reading it
                // from the fs and somehow making the file be there
                const SOUND_RAW: &[u8] = include_bytes!("../data/media/alarm.mp3");

                trace!("playing bundled sound");

                let sound_data: Cursor<_> = std::io::Cursor::new(SOUND_RAW);

                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let sink = Sink::try_new(&stream_handle).unwrap();
                sink.append(
                    Decoder::new(sound_data).expect("could not decode the bundled alarm sound"),
                );
                sink.sleep_until_end();

                debug!("played bundled sound");
            });
        }
        #[cfg(feature = "desktop")]
        {
            let mut notify = notify_rust::Notification::new();

            notify.appname(env!("CARGO_BIN_NAME"));

            // see [FreeDesktop Sound Naming Specification](http://0pointer.de/public/sound-naming-spec.html)
            // a sound exists for our use-case
            //
            // NOTE: sadly, notify_rust does not (yet) support KDE plasma, because
            // they have a weird way of making sounds and notifications in general
            // work. At least we get a little notification.

            // only play this when we don't use built in sound, this
            // isn't as consistent
            #[cfg(not(feature = "sound"))]
            notify.sound_name("alarm-clock-elapsed");

            // The user sets the time with the expectation to be notified, but it's
            // not like the moon is crashing into the earth
            notify.urgency(notify_rust::Urgency::Normal);

            // We don't need to have it be displayed for ever, the TUI shows that the time is up
            // (100%) already.
            notify.timeout(notify_rust::Timeout::Default);

            notify.summary(&format!(
                "Your countdown of {} is up.",
                humantime::Duration::from(self.countdown.unwrap())
            ));
            // NOTE: this will only work on machines with a proper desktop, not
            // with things like WSL2 or a docker container. Therefore, it is behind
            // the desktop feature.
            let _ = notify.show().inspect_err(|e| {
                error!("could not notify of finished countup: {e}");
                debug!(": {e:#?}");
            });
        }
        Ok(())
    }
    fn beep() -> anyhow::Result<()> {
        print!("\x07");
        std::io::stdout().flush()?;
        Ok(())
    }
    fn partition(r: Rect) -> HashMap<&'static str, Rect> {
        let part = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(if r.width > 80 { 8 } else { 5 }),
            ])
            .split(r);
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let hlen_date: u16 = (f32::from(part[1].width) * 0.32) as u16;
        let subparts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(hlen_date),
                Constraint::Length(part[0].width - hlen_date),
            ])
            .split(part[0]);

        let timebarw_spaces = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(subparts[1]);

        HashMap::from([
            ("clockw", part[1]),
            ("timebarw", timebarw_spaces[0]),
            ("timebarw_label", timebarw_spaces[1]),
            ("datew", subparts[0]),
        ])
    }
}
