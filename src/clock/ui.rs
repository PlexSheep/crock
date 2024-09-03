use chrono::{DateTime, Local, SubsecRound, Timelike};
use libpt::log::{debug, error, trace};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, LineGauge, Padding, Paragraph};

use crate::clock::timebar::TimeBarLength;

use super::Clock;

pub const TIME_FORMAT: &str = "%H:%M:%S";

// TODO: make this a ringbuffer with a custom struct inside?
#[derive(Debug, Clone, PartialEq)]
pub struct Data {
    now: [DateTime<Local>; 2],
    fdate: [String; 2],
    ftime: [String; 2],
    timebar_ratio: [Option<f64>; 2],

    timebar_type: TimeBarLength,
    started_at: DateTime<Local>,

    idx: usize,
}

impl Data {
    pub fn new(timebar_type: TimeBarLength) -> Self {
        let mut this = Self {
            now: [DateTime::default(); 2],
            fdate: [String::new(), String::new()],
            ftime: [String::new(), String::new()],
            timebar_ratio: [Option::default(); 2],
            started_at: Local::now(),
            idx: usize::default(),

            timebar_type,
        };
        this.started_at = this.started_at.round_subsecs(0);
        this
    }
    pub fn update(
        &mut self,
        now: DateTime<Local>,
        fdate: String,
        ftime: String,
        timebar_ratio: Option<f64>,
    ) {
        self.idx ^= 1;
        self.now[self.idx] = now;
        self.fdate[self.idx] = fdate;
        self.ftime[self.idx] = ftime;
        self.timebar_ratio[self.idx] = timebar_ratio;
        #[cfg(debug_assertions)]
        if self.changed() {
            trace!("update with change: {:#?}", self);
        }
    }

    /// did the data change with the last update?
    #[must_use]
    #[inline]
    pub fn changed(&self) -> bool {
        //  the timebar ratio is discarded, so that we only render the ui when the time
        //  (second) changes
        self.fdate[0] != self.fdate[1] || self.ftime[0] != self.ftime[1]
    }

    #[must_use]
    #[inline]
    pub fn fdate(&self) -> &str {
        &self.fdate[self.idx]
    }

    #[must_use]
    #[inline]
    pub fn ftime(&self) -> &str {
        &self.ftime[self.idx]
    }

    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // why should it be okay to make this const? This is
                                           // a custom ringbuffer!
    pub fn now(&self) -> &DateTime<Local> {
        &self.now[self.idx]
    }

    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // no it's not const
    pub fn timebar_ratio(&self) -> Option<f64> {
        if self.timebar_type == TimeBarLength::Timer {
            return Some(0.0);
        }
        self.timebar_ratio[self.idx]
    }
}

pub fn timebarw<'a>(
    clock: &mut Clock,
    data: &Data,
    timebarw_padding: &[u16],
    inner_rect: Rect,
) -> Option<LineGauge<'a>> {
    if clock.timebar_len().is_some() {
        debug!("time bar ration: {:?}", data.timebar_ratio());
        let ratio = data.timebar_ratio().unwrap();

        if !clock.did_notify && (ratio - 1.0).abs() < 0.000_001 {
            if let Some(TimeBarLength::Countup(_)) = clock.timebar_len() {
                let _ = clock.notify().inspect_err(|e| {
                    error!("could not notify: {e}");
                    debug!("complete error: {e:#?}");
                });
                clock.did_notify = true;
            }
        }

        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let timebarw = LineGauge::default()
            .filled_style(if clock.did_notify {
                Style::default()
                    .slow_blink()
                    .bold()
                    .underlined()
                    .yellow()
                    .crossed_out()
            } else {
                Style::default().blue()
            })
            .unfilled_style(Style::default())
            .block(
                Block::default().padding(Padding::right(if inner_rect.width > 80 {
                    timebarw_padding[0]
                } else {
                    timebarw_padding[1]
                })),
            )
            .ratio(ratio);
        Some(timebarw)
    } else {
        None
    }
}

pub fn timebarw_label<'a>(
    clock: &Clock,
    data: &Data,
    timebarw_padding: &[u16],
    inner_rect: Rect,
) -> Option<Paragraph<'a>> {
    clock.timebar_len().map(|len| {
        let last_reset = clock.last_reset.unwrap().round_subsecs(0);
        let time_now = match clock.timebar_len().unwrap() {
            TimeBarLength::Countup(secs) => {
                if clock.did_notify {
                    humantime::Duration::from(chrono::Duration::seconds(secs).to_std().unwrap())
                } else {
                    humantime::Duration::from(
                        data.now()
                            .round_subsecs(0)
                            .signed_duration_since(last_reset)
                            .to_std()
                            .unwrap(),
                    )
                }
            }
            TimeBarLength::Hour => humantime::Duration::from(
                data.now()
                    .signed_duration_since(last_reset)
                    .to_std()
                    .unwrap(),
            ),
            _ => humantime::Duration::from(
                data.now()
                    .round_subsecs(0)
                    .signed_duration_since(last_reset)
                    .to_std()
                    .unwrap(),
            ),
        };
        let until = {
            // we need to cut off the seconds if we're not in custom and countup mode, otherwise,
            // the timestamp will not be correct. This fixes #17
            match len {
                TimeBarLength::Custom(_) | TimeBarLength::Countup(_) => last_reset,
                _ => last_reset.with_second(0).unwrap(),
            }
        }
        // BUG: seconds are sometimes a little too much, for
        // example with `-o` #17
        .checked_add_signed(len.into())
        .expect("could not calculate when the countdown finishes")
        .format(TIME_FORMAT);

        let text: String = match clock.timebar_len().unwrap() {
            TimeBarLength::Timer => format!("{} + {time_now}", data.started_at.format(TIME_FORMAT)),
            TimeBarLength::Countup(_) | TimeBarLength::Custom(_) => format!(
                "{time_now} / {len} | {} -> {until}",
                last_reset.format(TIME_FORMAT)
            ),
            _ => format!(
                "{time_now} / {len} | {} -> {until}",
                last_reset.with_second(0).unwrap().format(TIME_FORMAT)
            ),
        };

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(
                Block::default().padding(Padding::right(if inner_rect.width > 80 {
                    timebarw_padding[0]
                } else {
                    timebarw_padding[1]
                })),
            )
    })
}
