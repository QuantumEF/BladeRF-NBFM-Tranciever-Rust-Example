use circular_buffer::CircularBuffer;
use num::complex::{Complex32, ComplexFloat};

const BUF_SIZE: usize = 1024;

pub enum SqlState {
    Open,
    Closed,
}

impl SqlState {
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    pub fn toggle(&mut self) {
        *self = match self {
            SqlState::Open => SqlState::Closed,
            SqlState::Closed => SqlState::Open,
        }
    }
}

pub struct Squelch {
    sql_hystereis_counter: usize,
    threshold: f32,
    averaging_buffer: CircularBuffer<BUF_SIZE, Complex32>,
    timeout_reset_count: usize,
    state: SqlState,
}

impl Squelch {
    pub fn new(counter: usize, thresh: f32) -> Self {
        Self {
            sql_hystereis_counter: 0,

            threshold: thresh,
            averaging_buffer: CircularBuffer::<BUF_SIZE, Complex32>::new(),
            timeout_reset_count: counter,
            state: SqlState::Closed,
        }
    }

    // // True means the threshold is met
    // pub fn check(&mut self, sample: Complex32) -> bool {
    //     self.averaging_buffer.push_back(sample);

    //     let average = self.average();

    //     if average > self.threshold {
    //         log::info!("Opening: {average}");
    //         self.sql_hystereis_counter = self.timeout_reset_count;
    //         true
    //     } else {
    //         log::info!("Closing: {average}");

    //         false
    //     }
    // }

    // True means the threshold is met
    pub fn check(&mut self, sample: Complex32) -> bool {
        self.averaging_buffer.push_back(sample);
        if self.sql_hystereis_counter == 0 {
            let average = self.average();
            // log::info!("average: {average}");
            // println!("average: {average}");
            if average > self.threshold {
                log::info!("Opening: {average}");
                self.sql_hystereis_counter = self.timeout_reset_count;
                self.state = SqlState::Open;
            } else {
                log::info!("Closing: {average}");
                self.sql_hystereis_counter = self.timeout_reset_count;

                self.state = SqlState::Closed;
            }
        }
        self.sql_hystereis_counter = self.sql_hystereis_counter.saturating_sub(1);
        self.state.is_open()
    }

    pub fn average(&mut self) -> f32 {
        self.averaging_buffer.iter().map(|x| x.abs()).sum::<f32>() / BUF_SIZE as f32
    }

    // pub fn check(&mut self, samples: &[f32]) -> bool {
    //     if self.sql_hystereis_counter == 0 {
    //         let folded: f32 = samples.iter().sum();
    //         let total_sum = folded / samples.len() as f32;
    //         if total_sum > self.threshold {
    //             self.sql_hystereis_counter = self.timeout_reset_count;
    //             true
    //         } else {
    //             false
    //         }
    //     } else {
    //         true
    //     }
    // }
}
