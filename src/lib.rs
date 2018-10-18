#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

extern crate rand;

use rand::*;

#[derive(Debug, Copy, Clone)]
pub struct ExtractStats {
    pub loop_count: u32,
    pub group_iterations: Option<u32>,
}

pub trait StatisticalMethod<T>
where
    T: Clone + Copy,
{
    fn add(&mut self, rate: f32, payload: T) -> Outcome<T>;
    fn delete(&mut self, outcome: Outcome<T>);
    fn update(&mut self, outcome: Outcome<T>, new_rate: f32) -> Outcome<T>;
    fn extract<Random: Rng>(&self, &mut Random) -> (ExtractStats, T);
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Outcome<T> {
    idx: usize,
    group_idx: Option<usize>,
    rate: f32,
    //light_idx: usize,
    payload: T,
}

#[derive(Clone, Debug)]
pub struct RejectionMethod<T> {
    max_rate: f32,
    outcomes: Vec<Outcome<T>>,
}

impl<T> RejectionMethod<T> {
    pub fn new(max_rate: f32) -> Self {
        Self {
            max_rate: max_rate,
            outcomes: vec![],
        }
    }
}

impl<T> StatisticalMethod<T> for RejectionMethod<T>
where
    T: Copy + Clone,
{
    fn add(&mut self, rate: f32, payload: T) -> Outcome<T> {
        if rate > self.max_rate || rate == 0.0 {
            // todo: clamp rate, or something
            panic!("Invalid rate provided in `add`");
        }

        let outcome = Outcome {
            payload: payload,
            group_idx: None,
            rate: rate,
            idx: self.outcomes.len(),
        };

        self.outcomes.push(outcome);
        outcome
    }

    fn delete(&mut self, outcome: Outcome<T>) {
        self.outcomes.swap_remove(outcome.idx);
        if self.outcomes.len() != 0 {
            self.outcomes[outcome.idx].idx = outcome.idx;
        }
    }

    fn update(&mut self, outcome: Outcome<T>, new_rate: f32) -> Outcome<T> {
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

    fn extract<Random: Rng>(&self, rng: &mut Random) -> (ExtractStats, T) {
        let mut loop_count = 0;
        loop {
            loop_count += 1;
            let rand = rng.gen_range::<f32>(0.0, self.outcomes.len() as f32);
            let rand_idx = rand.floor();
            let rand_rate = (rand - rand_idx) * self.max_rate;

            let outcome = &self.outcomes[rand_idx as usize];

            if outcome.rate >= rand_rate {
                return (
                    ExtractStats {
                        loop_count,
                        group_iterations: None,
                    },
                    outcome.payload,
                );
            }
        }
    }
}

#[derive(Debug)]
pub struct CompositeRejectionMethod<T> {
    groups: Vec<RejectionMethod<T>>,
    sum_rates: Vec<f32>,
    total_rate: f32,
    constant: f32,
    max: f32,
}

impl<T> CompositeRejectionMethod<T> {
    pub fn new(max: f32, constant: f32) -> Self {
        if constant <= 1.0 {
            panic!("Invalid constant");
        }

        let group_count = max.log(constant).ceil() as usize;
        let mut groups = vec![];

        for exponent in 0..group_count {
            groups.push(RejectionMethod::new(max / constant.powf(exponent as f32)));
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

impl<T> StatisticalMethod<T> for CompositeRejectionMethod<T>
where
    T: Copy + Clone,
{
    fn add(&mut self, rate: f32, payload: T) -> Outcome<T> {
        let group_idx = self.find_group_idx(rate);

        let mut outcome = self.groups[group_idx].add(rate, payload);
        self.sum_rates[group_idx] += rate;
        self.total_rate += rate;

        outcome.group_idx = Some(group_idx);
        outcome
    }

    fn delete(&mut self, outcome: Outcome<T>) {
        if let Some(group_idx) = outcome.group_idx {
            self.sum_rates[group_idx] -= outcome.rate;
            self.total_rate -= outcome.rate;

            self.groups[group_idx].delete(outcome);
        }
    }

    fn update(&mut self, outcome: Outcome<T>, new_rate: f32) -> Outcome<T> {
        let new_group_idx = self.find_group_idx(new_rate);

        if let Some(old_group_idx) = outcome.group_idx {
            let delta_rate = new_rate - outcome.rate;

            self.total_rate += delta_rate;

            let mut outcome = if new_group_idx == old_group_idx {
                // group stayed the same, just update
                self.sum_rates[new_group_idx] += delta_rate;
                self.groups[new_group_idx].update(outcome, new_rate)
            } else {
                // group changed, remove from old group
                self.sum_rates[old_group_idx] -= outcome.rate;
                self.groups[old_group_idx].delete(outcome);

                // add to new group
                self.sum_rates[new_group_idx] += new_rate;
                self.groups[new_group_idx].add(new_rate, outcome.payload)
            };

            outcome.group_idx = Some(new_group_idx);
            return outcome;
        } else {
            panic!("Outcome must have a group idx set");
        }
    }

    fn extract<Random: Rng>(&self, rng: &mut Random) -> (ExtractStats, T) {
        let u = rng.gen::<f32>();
        let mut rand = u * self.total_rate;
        let mut iterations = 0;
        for (idx, g) in self.groups.iter().enumerate() {
            if self.sum_rates[idx] > rand {
                let mut r = g.extract(rng);
                r.0.group_iterations = Some(iterations);
                return r;
            }

            iterations += 1;

            rand = rand - self.sum_rates[idx];
        }

        panic!("Shouldn't be able to reach here, algorithm invariant breached");
    }
}

#[derive(Clone)]
pub struct AliasMethod {
    pub alias: Vec<u32>,
    pub probability: Vec<f32>,
}

impl AliasMethod {
    pub fn new(mut list: Vec<f32>) -> AliasMethod {
        let mut sum = 0.0;

        for p in list.iter() {
            sum += p;
        }

        let list_len = list.len() as f32;

        for p in list.iter_mut() {
            *p *= list_len / sum;
        }

        let mut small = Vec::new();
        let mut large = Vec::new();

        small.resize(list.len(), 0);
        large.resize(list.len(), 0);

        let mut num_small = 0;
        let mut num_large = 0;

        for k in 0..list.len() {
            let i = list.len() - k - 1;

            if list[i] < 1.0 {
                small[num_small] = i;
                num_small += 1;
            } else {
                large[num_large] = i;
                num_large += 1;
            }
        }

        let mut alias = AliasMethod {
            alias: vec![0; list.len()],
            probability: vec![0.0; list.len()],
        };

        while num_small != 0 && num_large != 0 {
            num_small -= 1;
            num_large -= 1;

            let a = small[num_small];
            let g = large[num_large];

            alias.probability[a] = list[a];
            alias.alias[a] = g as u32;

            list[g] = list[g] + list[a] - 1.0;

            if list[g] < 1.0 {
                small[num_small] = g;
                num_small += 1;
            } else {
                large[num_large] = g;
                num_large += 1;
            }
        }

        for k in 0..num_large {
            alias.probability[large[k]] = 1.0
        }

        for k in 0..num_small {
            alias.probability[small[k]] = 1.0
        }

        alias
    }

    pub fn find_index(&self, u0: f32, u1: f32) -> usize {
        let idx = (self.alias.len() as f32 * u0) as usize;
        if u1 < self.probability[idx] {
            idx
        } else {
            self.alias[idx] as usize
        }
    }
}
