use crate::view::{ValueView, cmp_view};

use super::*;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        cmp_view(self, other).is_eq()
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        cmp_view(self, other)
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head().hash(state);
        self.payload().hash(state);
    }
}
