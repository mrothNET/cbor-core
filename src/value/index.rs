use std::ops::{Index, IndexMut};

use crate::Value;

impl<'a, I: Into<crate::ValueKey<'a>>> Index<I> for Value {
    type Output = Value;

    fn index(&self, index: I) -> &Value {
        self.get(index)
            .expect("value should be an array or map containing the given key")
    }
}

impl<'a, I: Into<crate::ValueKey<'a>>> IndexMut<I> for Value {
    fn index_mut(&mut self, index: I) -> &mut Value {
        self.get_mut(index)
            .expect("value should be an array or map containing the given key")
    }
}
