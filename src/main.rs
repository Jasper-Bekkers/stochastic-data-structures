extern crate rand;

use rand::*;

struct Outcome {
    payload: usize,
    idx: usize,
    rate: f32,
}

struct RejectionMethod {
    last_inserted: usize,
    max_rate: f32,
    outcomes: Vec<Outcome>,
}

impl RejectionMethod {
    fn new(max_rate: f32) -> Self {
        Self {
            last_inserted: 0,
            max_rate: max_rate,
            outcomes: vec![],
        }
    }

    fn add(&mut self, rate: f32, payload: usize) {
        if rate > self.max_rate {
            // todo: clamp rate, or something
            panic!("Invalid rate");
        }

        let outcome = Outcome {
            payload: payload,
            rate: rate,
            idx: self.outcomes.len(),
        };

        self.outcomes.push(outcome);
    }

    fn delete(&mut self, outcome_idx: usize) {
        self.outcomes.swap_remove(outcome_idx);
    }

    fn update(&mut self, outcome_idx: usize, rate: f32) {
        if rate == 0.0 && self.outcomes[outcome_idx].rate > 0.0 {
            // self.delete(outcome_idx);
            self.outcomes.swap_remove(outcome_idx);
        } else if (rate > 0.0 && self.outcomes[outcome_idx].rate == 0.0) {
            //  self.add(outcome_idx);
        }
        let mut outcome = &mut self.outcomes[outcome_idx];

        outcome.rate = rate;
    }

    fn extract<T: Rng>(&mut self, rng: &mut T) -> (u32, usize) {
        let mut loop_count = 0;
        loop {
            loop_count += 1;
            let k = rng.gen_range::<f32>(0.0, self.outcomes.len() as f32);
            let rand_idx = k.floor();
            let rand_rate = (k - rand_idx) * self.max_rate;

            let outcome = &self.outcomes[rand_idx as usize];

            if outcome.rate >= rand_rate {
                return (loop_count, outcome.payload);
            }
        }
    }
}

fn main() {
    let mut rj = RejectionMethod::new(3.0);

    rj.add(1.0, 0);
    rj.add(2.0, 1);

    let mut rng = thread_rng();

    let mut max_loop = 0;

    let mut list = [0.0, 0.0];
    let iter_count = 10000;

    for x in 0..iter_count {
        let res = rj.extract(&mut rng);
        list[res.1] += 1.0 / iter_count as f32;
        max_loop = max_loop.max(res.0);
    }

    println!("{:?}", list);

    println!("Max loop count {}", max_loop);
}
