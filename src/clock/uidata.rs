use libpt::log::trace;

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
    #[allow(clippy::missing_const_for_fn)] // no it's not const
    pub fn timebar_ratio(&self) -> Option<f64> {
        self.timebar_ratio[self.data_idx]
    }
}
