use tree_sitter::StreamingIterator;
pub trait StreamingIteratorExt<T> {
    fn collect(self) -> Vec<T>;
}

impl<T: Clone, Iter: StreamingIterator<Item = T>> StreamingIteratorExt<T> for Iter {
    fn collect(mut self) -> Vec<T> {
        let mut result = vec![];
        while let Some(item) = self.next() {
            result.push(item.clone());
        }
        result
    }
}
