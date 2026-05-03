use crate::view::{ValueView, cmp_view};

use super::*;

impl<'a> PartialEq for Value<'a> {
    fn eq(&self, other: &Self) -> bool {
        cmp_view(self, other).is_eq()
    }
}

impl<'a> Eq for Value<'a> {}

impl<'a> Ord for Value<'a> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        cmp_view(self, other)
    }
}

impl<'a> PartialOrd for Value<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Hash for Value<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head().hash(state);
        self.payload().hash(state);
    }
}
