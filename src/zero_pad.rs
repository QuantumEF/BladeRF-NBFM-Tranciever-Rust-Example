use std::iter::repeat;

pub struct Pad<T: Clone> {
    item: T,
    count: usize,
}

impl<T: Clone> Pad<T> {
    pub fn new(item: T, count: usize) -> Self {
        Self { item, count }
    }

    pub fn pad_sample(&self, sample: T) -> impl Iterator<Item = T> {
        [sample]
            .into_iter()
            .chain(repeat(self.item.clone()).take(self.count))
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::Pad;

    #[test]
    fn pad_test() {
        let x = [1, 2, 3, 4, 5];

        let pad = Pad::new(0, 3);

        let y = x.into_iter().flat_map(|s| pad.pad_sample(s)).collect_vec();

        assert_eq!(
            &y,
            &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0]
        );
    }
}
