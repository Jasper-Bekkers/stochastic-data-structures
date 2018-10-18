#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

extern crate rand;
extern crate stats;

use rand::*;
use stats::*;

trait StatisticalMethod {
    fn add(&mut self, rate: f32, light_idx: usize) -> Outcome;
    fn delete(&mut self, outcome: Outcome);
    fn update(&mut self, outcome: Outcome, new_rate: f32) -> Outcome;
    fn extract<T: Rng>(&self, &mut T) -> (u32, usize);
}

#[derive(Default, Clone, Copy, Debug)]
struct Outcome {
    idx: usize,
    group_idx: Option<usize>,
    rate: f32,
    light_idx: usize,
}

#[derive(Clone, Debug)]
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
}

impl StatisticalMethod for RejectionMethod {
    fn add(&mut self, rate: f32, light_idx: usize) -> Outcome {
        if rate > self.max_rate || rate == 0.0 {
            // todo: clamp rate, or something
            panic!("Invalid rate provided in `add`");
        }

        let outcome = Outcome {
            light_idx: light_idx,
            group_idx: None,
            rate: rate,
            idx: self.outcomes.len(),
        };

        self.outcomes.push(outcome);
        outcome
    }

    fn delete(&mut self, outcome: Outcome) {
        self.outcomes.swap_remove(outcome.idx);
        if self.outcomes.len() != 0 {
            self.outcomes[outcome.idx].idx = outcome.idx;
        }
    }

    fn update(&mut self, outcome: Outcome, new_rate: f32) -> Outcome {
        // if new_rate == 0.0 && self.outcomes[outcome_idx].rate > 0.0 {
        //     self.delete(outcome_idx);
        // } else if (new_rate > 0.0 && self.outcomes[outcome_idx].rate == 0.0) {
        // }

        if new_rate == 0.0 {
            panic!("Invalid rate provided in `update`");
        }

        let mut outcome = &mut self.outcomes[outcome.idx];
        outcome.rate = new_rate;
        *outcome
    }

    fn extract<T: Rng>(&self, rng: &mut T) -> (u32, usize) {
        let mut loop_count = 0;
        loop {
            loop_count += 1;
            let rand = rng.gen_range::<f32>(0.0, self.outcomes.len() as f32);
            let rand_idx = rand.floor();
            let rand_rate = (rand - rand_idx) * self.max_rate;

            let outcome = &self.outcomes[rand_idx as usize];

            if outcome.rate >= rand_rate {
                return (loop_count, outcome.light_idx);
            }
        }
    }
}

#[derive(Debug)]
struct CompositeRejectionMethod {
    groups: Vec<RejectionMethod>,
    sum_rates: Vec<f32>,
    total_rate: f32,
    constant: f32,
    max: f32,
}

impl CompositeRejectionMethod {
    fn new(max: f32, constant: f32) -> Self {
        if constant <= 1.0 {
            panic!("Invalid constant");
        }
        let mut groups = vec![];
        let group_count = max.log(constant).ceil() as usize;

        let mut exponent = 0.0;

        for x in 0..group_count {
            groups.push(RejectionMethod::new(max / constant.powf(exponent)));
            exponent += 1.0
        }

        Self {
            groups: groups,
            sum_rates: vec![0.0; group_count],
            total_rate: 0.0,
            constant: constant,
            max: max,
        }
    }

    fn find_group_idx(&self, rate: f32) -> usize {
        (self.max / rate).log(self.constant).floor() as usize
    }
}

impl StatisticalMethod for CompositeRejectionMethod {
    fn add(&mut self, rate: f32, light_idx: usize) -> Outcome {
        let group_idx = self.find_group_idx(rate);

        let mut outcome = self.groups[group_idx].add(rate, light_idx);
        self.sum_rates[group_idx] += rate;
        self.total_rate += rate;

        outcome.group_idx = Some(group_idx);
        outcome
    }

    fn delete(&mut self, outcome: Outcome) {}

    fn update(&mut self, outcome: Outcome, new_rate: f32) -> Outcome {
        let new_group_idx = self.find_group_idx(new_rate);

        if let Some(old_group_idx) = outcome.group_idx {
            let delta_rate = new_rate - outcome.rate;

            self.total_rate += delta_rate;

            let mut outcome = if new_group_idx == old_group_idx {
                self.sum_rates[new_group_idx] += delta_rate;
                self.groups[new_group_idx].update(outcome, new_rate)
            } else {
                self.sum_rates[old_group_idx] -= outcome.rate;
                self.groups[old_group_idx].delete(outcome);

                self.sum_rates[new_group_idx] += new_rate;
                self.groups[new_group_idx].add(new_rate, outcome.light_idx)
            };

            outcome.group_idx = Some(new_group_idx);
            return outcome;
        } else {
            panic!("Outcome must have a group idx set");
        }
    }

    fn extract<T: Rng>(&self, rng: &mut T) -> (u32, usize) {
        let u = rng.gen::<f32>();
        let mut rand = u * self.total_rate;
        for (idx, g) in self.groups.iter().enumerate() {
            if self.sum_rates[idx] > rand {
                return g.extract(rng);
            }

            rand = rand - self.sum_rates[idx];
        }

        panic!("Shouldn't be able to reach here, algorithm invariant breached");
    }
}

fn main() {
    let max_rate = 30000.0 * 2.0;
    let mut rj = CompositeRejectionMethod::new(max_rate, 2.0);
    //let mut rj = RejectionMethod::new(max_rate);

    rj.add(1.0, 0);
    rj.add(1.0, 1);
    let update_me = rj.add(1.0, 2);
    rj.add(1.0, 3);
    rj.add(1.0, 4);
    rj.add(1.0, 5);
    rj.add(1.0, 6);

    let update_2 = rj.update(update_me, 2.0);

    rj.update(update_2, 10.0);

    // println!("{:#?}", rj);

    let mut rng = thread_rng();

    let mut max_loop = 0;

    let mut list = [0.0; 7];
    let iter_count = 100000;

    let mut counts = vec![];

    for _x in 0..iter_count {
        let res = rj.extract(&mut rng);
        list[res.1] += 1.0 / iter_count as f32;
        counts.push(res.0 as f64);
        max_loop = max_loop.max(res.0);
    }

    println!("{:?}", list);

    println!(
        "Loop count, max: {}, mean: {}, median: {}, stddev: {}, variance: {}",
        max_loop,
        mean(counts.iter().map(|x| *x)),
        median(counts.iter().map(|x| *x)).unwrap(),
        stddev(counts.iter().map(|x| *x)),
        variance(counts.iter().map(|x| *x))
    );

    //println!("{:?}", counts);
}
