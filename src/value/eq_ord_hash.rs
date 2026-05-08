use std::{
    cmp,
    hash::{Hash, Hasher},
};

use crate::{
    Value,
    view::{ValueView, cmp_view},
};

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        cmp_view(self, other).is_eq()
    }
}

impl Eq for Value<'_> {}

impl Ord for Value<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        cmp_view(self, other)
    }
}

impl PartialOrd for Value<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Value<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head().hash(state);
        self.payload().hash(state);
    }
}
