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

/// The two opacities the indicator has. Not `set_visible`, which would take
/// the row's height with it — see [`JobIndicator::new`].
const VISIBLE: f64 = 1.0;
const HIDDEN: f64 = 0.0;
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
        // button is what makes the thing look pressable, which a progress bar
        // has no affordance for. Not focusable — `Tab` switches panes here,
        // so there is no keyboard route to it and an invisible widget in the
        // focus chain would only be somewhere the focus could get lost.
        //
        // **Always in the layout**, and hidden by going transparent rather
        // than by `set_visible(false)`. A widget that comes and goes takes
        // the row's height with it, so the whole window twitched every time a
        // job started or ended — which the second testing round reported.
        // Transparent still occupies its space, so the bottom row has one
        // height for the life of the window.
        let button = gtk::Button::builder()
            .child(&bar)
            .has_frame(false)
            .opacity(0.0)
            .can_target(false)
            .can_focus(false)
            .build();
        button.add_css_class(CLASS_JOB_INDICATOR);

        JobIndicator { button, bar }
    }

    pub fn widget(&self) -> &gtk::Button {
        &self.button
    }

    /// Shows the indicator, at whatever the meter now says.
    ///
    /// The caption goes *inside* the bar rather than beside it, which is
    /// where the room is: a percentage and a byte count next to a bar would
    /// be a second widget in a row that has none to spare.
    pub fn show(&self, meter: &Meter) {
        self.bar.set_fraction(meter.fraction());
        self.bar.set_text(Some(&meter.caption()));
        // Only when it is not already up. A job sends tens of thousands of
        // progress events — 55 080 for the end-to-end suite's copy — and
        // setting a property to what it already holds is work asked for
        // fifty-five thousand times.
        if self.button.opacity() != VISIBLE {
            self.button.set_opacity(VISIBLE);
            self.button.set_can_target(true);
        }
    }

    /// Takes it away, which is what the end of a job looks like.
    ///
    /// Transparent, not gone: the space stays reserved, and a bar showing the
    /// last job's percentage forever would be worse than no bar. `can_target`
    /// goes with it, so an invisible button cannot be clicked.
    pub fn hide(&self) {
        self.button.set_opacity(HIDDEN);
        self.button.set_can_target(false);
    }
}
