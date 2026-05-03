use std::ops::{Index, IndexMut};

use crate::Value;

impl<'a, 'b, I: Into<crate::ValueKey<'b>>> Index<I> for Value<'a> {
    type Output = Value<'a>;

    fn index(&self, index: I) -> &Value<'a> {
        self.get(index)
            .expect("value should be an array or map containing the given key")
    }
}

impl<'a, 'b, I: Into<crate::ValueKey<'b>>> IndexMut<I> for Value<'a> {
    fn index_mut(&mut self, index: I) -> &mut Value<'a> {
        self.get_mut(index)
            .expect("value should be an array or map containing the given key")
    }
}
