use std::cell::Cell;

pub struct Keep1InN<const N: usize> {
    counter: Cell<usize>,
}

impl<const N: usize> Keep1InN<N> {
    pub fn new() -> Self {
        Keep1InN {
            counter: Cell::new(1),
        }
    }

    pub fn test_keep(&self) -> bool {
        let prev_counter = self.counter.get();
        let keep = prev_counter == 1;
        if prev_counter == N {
            self.counter.set(1);
        } else {
            self.counter.set(prev_counter + 1);
        };
        keep
    }

    pub fn reset(&self) {
        self.counter.set(0);
    }
}

impl<const N: usize> Default for Keep1InN<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::Keep1InN;

    #[test]
    fn keep_test() {
        let x = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let dec = Keep1InN::<2>::new();
        let y = x.iter().copied().filter(|_| dec.test_keep()).collect_vec();
        assert_eq!(y, [1, 3, 5, 7, 9])
    }
}
