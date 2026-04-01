use super::*;

impl<I: Into<Value>> Index<I> for Value {
    type Output = Value;

    fn index(&self, index: I) -> &Value {
        self.get(index)
            .expect("value should be an array or map containing the given key")
    }
}

impl<I: Into<Value>> IndexMut<I> for Value {
    fn index_mut(&mut self, index: I) -> &mut Value {
        self.get_mut(index)
            .expect("value should be an array or map containing the given key")
    }
}
