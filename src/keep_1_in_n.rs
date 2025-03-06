use std::cell::Cell;

pub struct Keep1InN<const N: usize> {
    counter: Cell<usize>,
}

impl<const N: usize> Keep1InN<N> {
    pub fn new() -> Self {
        Keep1InN {
            counter: Cell::new(0),
        }
    }

    pub fn test_keep(&self) -> bool {
        let prev_counter = self.counter.get();
        let keep = prev_counter == 0;
        if prev_counter == N {
            self.counter.set(0);
        } else {
            self.counter.set(prev_counter + 1);
        };
        keep
    }
}

impl<const N: usize> Default for Keep1InN<N> {
    fn default() -> Self {
        Self::new()
    }
}
