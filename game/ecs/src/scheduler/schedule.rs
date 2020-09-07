use crate::scheduler::System;
use std::sync::{Arc, Mutex};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    pub(crate) systems: Vec<Arc<Mutex<Box<dyn System>>>>,
}
