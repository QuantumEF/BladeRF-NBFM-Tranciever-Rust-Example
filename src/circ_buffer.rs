use std::{cell::Cell, iter};

pub struct MyCircBuf<T, const N: usize> {
    buf: [T; N],
    pos: Cell<usize>,
}

impl<T: Copy, const N: usize> MyCircBuf<T, N> {
    pub fn new(buf: [T; N]) -> MyCircBuf<T, N> {
        MyCircBuf {
            buf,
            pos: Cell::new(0),
        }
    }

    pub fn step(&self) {
        if self.pos.get() == (N - 1) {
            self.pos.set(0);
        } else {
            // self.pos.update(|x| x + 1);
            let newpos = self.pos.get() + 1;
            self.pos.set(newpos);
        }
    }

    pub fn step_by(&self, step_size: usize) -> impl Iterator<Item = T> + use<'_, T, N> {
        let mut counter = 0;
        iter::from_fn(move || {
            if counter >= N {
                None
            } else {
                let index = (counter + self.pos.get()) % self.buf.len();
                let item = self.buf[index];
                counter += step_size;
                Some(item)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircBuf;

    #[test]
    fn circ_buf_step1() {
        let buffer = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        let circ = MyCircBuf::new(buffer);

        circ.step();
        circ.step();

        let out: Vec<_> = circ.step_by(1).collect();
        let out2: Vec<_> = circ.step_by(2).collect();

        println!("Output : {out:#?}");
        println!("Output2: {out2:#?}");
    }
}
