use chrono::{DateTime, Local, SubsecRound};
use libpt::log::{debug, error, trace};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, LineGauge, Padding, Paragraph};

use crate::clock::timebar::TimeBarLength;

use super::Clock;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiData {
    now: [DateTime<Local>; 2],
    fdate: [String; 2],
    ftime: [String; 2],
    timebar_ratio: [Option<f64>; 2],

    data_idx: usize,
}

impl UiData {
    pub fn update(
        &mut self,
        now: DateTime<Local>,
        fdate: String,
        ftime: String,
        timebar_ratio: Option<f64>,
    ) {
        self.data_idx ^= 1;
        self.now[self.data_idx] = now;
        self.fdate[self.data_idx] = fdate;
        self.ftime[self.data_idx] = ftime;
        self.timebar_ratio[self.data_idx] = timebar_ratio;
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
        &self.fdate[self.data_idx]
    }

    #[must_use]
    #[inline]
    pub fn ftime(&self) -> &str {
        &self.ftime[self.data_idx]
    }

    #[must_use]
    #[inline]
    pub fn now(&self) -> &DateTime<Local> {
        &self.now[self.data_idx]
    }

    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // no it's not const
    pub fn timebar_ratio(&self) -> Option<f64> {
        self.timebar_ratio[self.data_idx]
    }
}

pub fn timebarw<'a>(
    clock: &mut Clock,
    data: &UiData,
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
    data: &UiData,
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
            _ => humantime::Duration::from(
                data.now()
                    .round_subsecs(0)
                    .signed_duration_since(last_reset)
                    .to_std()
                    .unwrap(),
            ),
        };
        Paragraph::new(format!("{time_now} / {len}"))
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
