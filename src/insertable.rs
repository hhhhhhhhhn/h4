use std::iter::Peekable;
use std::collections::VecDeque;

pub struct InsertableIterator<'a, T> {
    inserted: VecDeque<T>,
    iter: Peekable<Box<dyn Iterator<Item = T> + 'a>>,
}

impl<T> InsertableIterator<'_, T> {
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

    pub fn new<'a>(iter: Box<dyn Iterator<Item = T> + 'a>) -> InsertableIterator<'a, T> {
        return InsertableIterator {
            iter: iter.peekable(),
            inserted: VecDeque::new(),
        }
    }
}

impl<T> Iterator for InsertableIterator<'_, T> {
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

impl<'a, T> std::convert::From<Box<dyn Iterator<Item = T> + 'a>> for InsertableIterator<'a, T> {
    fn from(iter: Box<dyn Iterator<Item = T> + 'a>) -> Self {
        return Self::new(iter);
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

    #[test]
    fn test_from_string() {
        let string = "A very cool string".to_string();
        let boxed: Box<dyn Iterator<Item = char>> = Box::new(string.chars());

        let mut iterator = InsertableIterator::from(boxed);

        while iterator.next() != None {}
    }
}
