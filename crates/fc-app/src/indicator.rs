//! The corner of the window that says a job is still running.
//!
//! `Background` on the progress window leaves the job running and takes the
//! only thing on screen that referred to it away — so a copy of ten thousand
//! files carried on with nothing to show for it, and no way back to cancel
//! it. This is that something: a bar at the end of the command line's row,
//! there only while a job is running, and clickable to get the window back.
//!
//! It renders from the same [`Meter`](crate::progress::Meter) the window
//! does, so "42 %" here and "42 %" there are one calculation rather than two
//! that drift.

use gtk::prelude::*;

use crate::constants::{CLASS_JOB_INDICATOR, INDICATOR_WIDTH};
use crate::progress::Meter;

/// The bar, and the button that is really the clickable part.
pub struct JobIndicator {
    button: gtk::Button,
    bar: gtk::ProgressBar,
}

impl JobIndicator {
    pub fn new() -> Self {
        let bar = gtk::ProgressBar::builder()
            .show_text(true)
            .width_request(INDICATOR_WIDTH)
            .valign(gtk::Align::Center)
            .build();

        // A button around the bar rather than a click handler on it: the
        // button is what makes the thing look pressable and reachable by
        // keyboard, and a progress bar has no such affordance of its own.
        let button = gtk::Button::builder()
            .child(&bar)
            .has_frame(false)
            .visible(false)
            .build();
        button.add_css_class(CLASS_JOB_INDICATOR);

        JobIndicator { button, bar }
    }

    pub fn widget(&self) -> &gtk::Button {
        &self.button
    }

    /// Shows the indicator, at whatever the meter now says.
    pub fn show(&self, meter: &Meter) {
        self.bar.set_fraction(meter.fraction());
        self.bar.set_text(Some(&meter.caption()));
        // Only when it is not already showing. A job sends tens of thousands
        // of progress events — 55 080 for the end-to-end suite's copy — and
        // making a widget visible that already is means asking GTK to lay the
        // row out again each time.
        if !self.button.is_visible() {
            self.button.set_visible(true);
        }
    }

    /// Takes it away, which is what the end of a job looks like.
    pub fn hide(&self) {
        self.button.set_visible(false);
    }
}
