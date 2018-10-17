extern crate rand;

use rand::*;

struct Outcome {
    light_idx: usize,
    idx: usize,
    rate: f32,
}

struct RejectionMethod {
    max_rate: f32,
    outcomes: Vec<Outcome>,
}

impl RejectionMethod {
    fn new(max_rate: f32) -> Self {
        Self {
            max_rate: max_rate,
            outcomes: vec![],
        }
    }

    fn add(&mut self, rate: f32, light_idx: usize) {
        if rate > self.max_rate {
            // todo: clamp rate, or something
            panic!("Invalid rate");
        }

        let outcome = Outcome {
            light_idx: light_idx,
            rate: rate,
            idx: self.outcomes.len(),
        };

        self.outcomes.push(outcome);
    }

    fn delete(&mut self, outcome_idx: usize) {
        self.outcomes.swap_remove(outcome_idx);
        if self.outcomes.len() != 0 {
            self.outcomes[outcome_idx].idx = outcome_idx;
        }
    }

    fn update(&mut self, outcome_idx: usize, rate: f32) {
        // if rate == 0.0 && self.outcomes[outcome_idx].rate > 0.0 {
        //     self.delete(outcome_idx);
        // } else if (rate > 0.0 && self.outcomes[outcome_idx].rate == 0.0) {

        // }
        let mut outcome = &mut self.outcomes[outcome_idx];

        outcome.rate = rate;
    }

    fn extract<T: Rng>(&mut self, rng: &mut T) -> (u32, usize) {
        let mut loop_count = 0;
        loop {
            loop_count += 1;
            let rand = rng.gen_range::<f32>(0.0, self.outcomes.len() as f32);
            let rand_idx = rand.floor();
            let rand_rate = (rand - rand_idx) * self.max_rate;

            let outcome = &self.outcomes[rand_idx as usize];

            if outcome.rate >= rand_rate {
                return (loop_count, outcome.idx);
            }
        }
    }
}

struct CompositeRejectionMethod {
    groups: Vec<RejectionMethod>,
    sum_rates: Vec<f32>,
    total_rate: f32,
    constant: f32,
    max: f32,
}

impl CompositeRejectionMethod {
    fn new(num_groups: usize, max: f32) -> Self {
        Self {
            groups: vec![],
            sum_rates: vec![],
            total_rate: 0.0,
            constant: 0.0,
            max: max,
        }
    }

    fn add(&mut self, rate: f32, light_idx: usize) {
        let group_idx = (self.max / rate).log(self.constant).floor() as usize;
        self.groups[group_idx].add(rate, light_idx);
        self.sum_rates[group_idx] += rate;
        self.total_rate += rate;
    }

    fn delete(&mut self, outcome_idx: usize) {}

    fn extract<T: Rng>(&mut self, rng: &mut T) -> (u32, usize) {
        let u = rng.gen::<f32>();
        let mut rand = u * self.total_rate;
        for (idx, g) in self.groups.iter_mut().enumerate() {
            if self.sum_rates[idx] >= rand {
                return g.extract(rng);
            }

            rand = rand - self.sum_rates[idx];
        }

        panic!("Shouldn't be able to reach here, algorithm invariant breached");
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
