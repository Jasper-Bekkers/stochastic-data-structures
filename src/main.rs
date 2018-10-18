#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

extern crate rand;
extern crate stats;

use rand::*;
use stats::*;

#[derive(Default, Clone, Debug)]
struct Outcome {
    light_idx: usize,
    idx: usize,
    rate: f32,
}

#[derive(Clone, Debug)]
struct RejectionMethod {
    max_rate: f32,
    outcomes: Vec<Outcome>,
}

trait StatisticalMethod {
    fn add(&mut self, rate: f32, light_idx: usize);
    fn delete(&mut self, outcome_idx: usize);
    fn update(&mut self, outcome_idx: usize, rate: f32);
    fn extract<T: Rng>(&self, &mut T) -> (u32, usize);
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
}

impl StatisticalMethod for CompositeRejectionMethod {
    fn add(&mut self, rate: f32, light_idx: usize) {
        let group_idx = (self.max / rate).log(self.constant).floor() as usize;
        self.groups[group_idx].add(rate, light_idx);
        self.sum_rates[group_idx] += rate;
        self.total_rate += rate;
    }

    fn delete(&mut self, outcome_idx: usize) {}
    fn update(&mut self, outcome_idx: usize, rate: f32) {}
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
    rj.add(2.0, 1);
    rj.add(1.0, 2);
    rj.add(10.0, 3);
    rj.add(100.0, 4);
    rj.add(1000.0, 5);
    rj.add(10000.0, 6);

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
