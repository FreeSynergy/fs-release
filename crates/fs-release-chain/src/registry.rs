//! Pattern: Registry (command name to command; the one place that fixes the order of the chain).

use crate::steps::{
    Bump, Check, Gates, MergeFf, Push, Release, Report, Review, Sign, Step, Tag, WaitCi,
};

/// The commands in chain order. The chain file has to list exactly these (a test keeps both equal).
#[must_use]
pub fn all() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(Gates),
        Box::new(Review),
        Box::new(Push),
        Box::new(WaitCi),
        Box::new(MergeFf),
        Box::new(Tag),
        Box::new(Release),
        Box::new(Bump),
        Box::new(Sign),
        Box::new(Check),
        Box::new(Report),
    ]
}

/// One command by name.
#[must_use]
pub fn find(name: &str) -> Option<Box<dyn Step>> {
    all().into_iter().find(|s| s.meta().name == name)
}
