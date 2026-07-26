use std::collections::VecDeque;

/// Iterator wrapper that allows peeking an arbitrary distance into the future.
#[derive(Debug, Clone)]
pub struct NPeekable<I: Iterator> {
    iter: I,
    buffer: VecDeque<I::Item>,
}

impl<I, T> NPeekable<I>
where
    I: Iterator<Item = T>,
    T: Copy,
{
    pub const fn new(iter: I) -> NPeekable<I> {
        NPeekable {
            iter,
            buffer: VecDeque::new(),
        }
    }

    /// Peeks a character at the given offset (`0` is the next character).
    pub fn peek(&mut self, offset: usize) -> Option<T> {
        while self.buffer.len() <= offset {
            if let Some(t) = self.iter.next() {
                self.buffer.push_back(t);
            } else {
                return None;
            }
        }

        Some(self.buffer[offset])
    }
}

impl<I, T> Iterator for NPeekable<I>
where
    I: Iterator<Item = T>,
    T: Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_front().or_else(|| self.iter.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();

        (
            lower + self.buffer.len(),
            upper.map(|x| x + self.buffer.len()),
        )
    }
}

#[macro_export]
macro_rules! do_while {
    ($body:tt while $cond:expr) => {{
        let mut __first = true;

        while (__first || ($cond)) {
            __first = false;
            $body
        }
    }};
}
