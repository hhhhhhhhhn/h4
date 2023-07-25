use std::iter::Peekable;
use std::collections::VecDeque;

pub struct InsertableIterator<T> {
    inserted: VecDeque<T>,
    iter: Peekable<Box<dyn Iterator<Item = T>>>,
}

impl<T> InsertableIterator<T> {
    pub fn insert_elements(&mut self, input: Vec<T>) {
        // TODO: Make faster
        for el in input.into_iter().rev() {
            self.inserted.push_front(el);
        }
    }

    pub fn peek(&mut self) -> Option<&T> {
        if self.inserted.len() > 0 {
            return self.inserted.front();
        }
        else {
            return self.iter.peek();
        }
    }

    pub fn new(iter: Box<dyn Iterator<Item = T>>) -> Self {
        return Self {
            iter: iter.peekable(),
            inserted: VecDeque::new(),
        }
    }
}

impl<T> Iterator for InsertableIterator<T> {
    type Item = T;
    
    fn next(&mut self) -> Option<T> {
        if self.inserted.len() > 0 {
            return self.inserted.pop_front();
        }
        else {
            return self.iter.next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration() {
        let values = vec![0, 1, 2, 3, 4, 5];
        let iterator = values.into_iter();
        let mut insertable = InsertableIterator::new(Box::new(iterator));

        for i in 0..6 {
            let peeked = insertable.peek();
            assert_eq!(peeked, Some(&i));
            let consumed = insertable.next();
            assert_eq!(consumed, Some(i));
        }
    }

    #[test]
    fn test_insert() {
        let values = vec![3, 4, 5];
        let iterator = values.into_iter();
        let mut insertable = InsertableIterator::new(Box::new(iterator));
        insertable.insert_elements(vec![0, 1, 2]);

        for i in 0..6 {
            let peeked = insertable.peek();
            assert_eq!(peeked, Some(&i));
            let consumed = insertable.next();
            assert_eq!(consumed, Some(i));
        }
    }

    #[test]
    fn test_insert_mid_iter() {
        let values = vec![3, 4, 5];
        let iterator = values.into_iter();
        let mut insertable = InsertableIterator::new(Box::new(iterator));
        insertable.insert_elements(vec![0, 1, 2]);

        insertable.next();
        insertable.next();
        insertable.next();
        insertable.next();

        insertable.insert_elements(vec![0, 1, 2, 3]);

        for i in 0..6 {
            let peeked = insertable.peek();
            assert_eq!(peeked, Some(&i));
            let consumed = insertable.next();
            assert_eq!(consumed, Some(i));
        }
    }
}
