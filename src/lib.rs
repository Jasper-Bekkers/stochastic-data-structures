#![feature(integer_atomics)]

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
    fn extract<Random: Rng>(&self, rnd: &mut Random) -> (ExtractStats, T);
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Outcome<T> {
    idx: usize,
    group_idx: Option<usize>,
    rate: f32,
    pub payload: T,
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
    fn add(&mut self, mut rate: f32, payload: T) -> Outcome<T> {
        if rate > self.max_rate {
            // todo: clamp rate, or something
            panic!("Invalid rate provided in `add`");
        }

        if rate == 0.0 {
            rate = 0.0001;
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
        /*if self.outcomes.len() != 0 {
            self.outcomes[outcome.idx].idx = outcome.idx;
        }*/
    }

    fn update(&mut self, outcome: Outcome<T>, new_rate: f32) -> Outcome<T> {
        // if new_rate == 0.0 && self.outcomes[outcome_idx].rate > 0.0 {
        //     self.delete(outcome_idx);
        // } else if (new_rate > 0.0 && self.outcomes[outcome_idx].rate == 0.0) {
        // }

        if new_rate == 0.0 {
            panic!("Invalid rate provided in `update`");
        }

        let outcome = &mut self.outcomes[outcome.idx];
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

        if max <= 1.0 {
            panic!("Invalid max value");
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
        // clamp rate to 1.0 on the lower end so all the rates between 0
        // and 1 fall into the very first bucket
        (self.max / rate.max(1.0)).log(self.constant).floor() as usize
    }
}

impl<T> StatisticalMethod<T> for CompositeRejectionMethod<T>
where
    T: Copy + Clone,
{
    fn add(&mut self, rate: f32, payload: T) -> Outcome<T> {
        if rate > self.max {
            panic!("Rate out of range rate: {}, max rate: {}", rate, self.max);
        }

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

use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};

#[derive(Debug)]
pub struct AtomicOutcome {
    pub idx: usize,
    pub group_idx: usize,
    pub payload_and_rate: AtomicU64, // top 32 bits are payload, bottom 32 are the 'rate' as 24:8 fixed point
}

#[derive(Debug)]
pub struct AtomicRejectionMethod {
    max_rate: f32,
    capacity: usize,
    group_idx: usize,
    outcomes_len: AtomicUsize,
    outcomes: Vec<AtomicOutcome>,
}

impl AtomicRejectionMethod {
    pub fn new(capacity: usize, group_idx: usize, max_rate: f32) -> Self {
        let mut outcomes = vec![];
        for idx in 0..capacity {
            outcomes.push(AtomicOutcome {
                idx,
                group_idx,
                payload_and_rate: AtomicU64::new(0),
            })
        }

        Self {
            max_rate,
            outcomes_len: AtomicUsize::new(0usize),
            capacity,
            group_idx,
            outcomes,
        }
    }

    pub fn extract<Random: Rng>(&self, rng: &mut Random) -> (ExtractStats, u32) {
        let mut loop_count = 0;
        loop {
            loop_count += 1;
            let rand = rng.gen_range::<f32>(0.0, self.outcomes.len() as f32);
            let rand_idx = rand.floor();
            let rand_rate = (rand - rand_idx) * self.max_rate;

            let payload_and_rate = self.outcomes[rand_idx as usize]
                .payload_and_rate
                .load(Ordering::SeqCst);

            let rate = from_fixed((payload_and_rate & 0xffFFffFFu64) as u32);
            let payload = (payload_and_rate >> 32u64) as u32;

            if rate >= rand_rate {
                return (
                    ExtractStats {
                        loop_count,
                        group_iterations: None,
                    },
                    payload,
                );
            }
        }
    }
}

#[derive(Debug)]
pub struct AtomicCompositeRejectionMethod {
    groups: Vec<AtomicRejectionMethod>,
    sum_rates: Vec<AtomicU32>,
    total_rate: AtomicU32,
    constant: f32,
    max: f32,
}

impl AtomicCompositeRejectionMethod {
    pub fn new(max: f32, constant: f32, len: usize) -> Self {
        let group_count = max.log(constant).ceil() as usize;
        let mut groups = vec![];
        let mut sum_rates = vec![];
        for group_idx in 0..group_count {
            sum_rates.push(AtomicU32::new(0));
            groups.push(AtomicRejectionMethod::new(
                len,
                group_idx,
                max / constant.powf(group_idx as f32),
            ));
        }

        Self {
            groups,
            sum_rates,
            total_rate: AtomicU32::new(0),
            constant,
            max,
        }
    }

    fn find_group_idx(&self, rate: f32) -> usize {
        // clamp rate to 1.0 on the lower end so all the rates between 0
        // and 1 fall into the very first bucket
        (self.max / rate.max(1.0)).log(self.constant).floor() as usize
    }

    pub fn add(&self, rate: f32, payload: u32) -> usize {
        let group_idx = self.find_group_idx(rate);
        let fixed_rate = to_fixed(rate);

        self.sum_rates[group_idx].fetch_add(fixed_rate, Ordering::SeqCst);
        self.total_rate.fetch_add(fixed_rate, Ordering::SeqCst);

        let idx = self.groups[group_idx]
            .outcomes_len
            .fetch_add(1, Ordering::SeqCst);

        self.groups[group_idx].outcomes[idx].payload_and_rate.swap(
            ((payload as u64) << 32u64) | (fixed_rate as u64),
            Ordering::SeqCst,
        );

        ((fixed_rate as usize) << 32usize) | idx
    }

    pub fn update(&self, old_rate_and_outcome_idx: usize, new_rate: f32) {
        let new_group_idx = self.find_group_idx(new_rate);

        let old_rate = from_fixed((old_rate_and_outcome_idx >> 32usize) as u32);
        let outcome_idx = old_rate_and_outcome_idx & 0xffFFffFFusize;
        let old_group_idx = self.find_group_idx(old_rate);

        let delta_rate = new_rate - old_rate;
        let fixed_delta_rate = to_fixed(delta_rate);
        self.total_rate
            .fetch_add(fixed_delta_rate, Ordering::SeqCst);

        if new_group_idx == old_group_idx {
            // group stayed the same, just update
            self.sum_rates[new_group_idx].fetch_add(fixed_delta_rate, Ordering::SeqCst);

            let idx = self.groups[new_group_idx]
                .outcomes_len
                .fetch_add(1, Ordering::SeqCst);

            loop {
                let old_payload_and_rate = self.groups[old_group_idx].outcomes[outcome_idx]
                    .payload_and_rate
                    .load(Ordering::SeqCst);

                let result = self.groups[new_group_idx].outcomes[idx]
                    .payload_and_rate
                    .compare_exchange(
                        old_payload_and_rate,
                        (old_payload_and_rate & 0xffFFffFF_00000000) | (to_fixed(new_rate) as u64),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );

                if result.is_ok() {
                    break;
                }
            }
        } else {
            // group changed, remove from old group
            self.sum_rates[old_group_idx].fetch_sub(to_fixed(old_rate), Ordering::SeqCst);

            let swap_idx = self.groups[old_group_idx]
                .outcomes_len
                .fetch_sub(1, Ordering::SeqCst);

            let mut old_payload_and_rate = 0;

            loop {
                old_payload_and_rate = self.groups[old_group_idx].outcomes[outcome_idx]
                    .payload_and_rate
                    .load(Ordering::SeqCst);

                let swap_payload_and_rate = self.groups[old_group_idx].outcomes[swap_idx]
                    .payload_and_rate
                    .load(Ordering::SeqCst);

                let result = self.groups[old_group_idx].outcomes[outcome_idx]
                    .payload_and_rate
                    .compare_exchange(
                        old_payload_and_rate,
                        swap_payload_and_rate,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );

                if result.is_ok() {
                    break;
                }
            }

            // add to new group
            self.sum_rates[new_group_idx].fetch_add(to_fixed(new_rate), Ordering::SeqCst);

            let idx = self.groups[new_group_idx]
                .outcomes_len
                .fetch_add(1, Ordering::SeqCst);

            self.groups[new_group_idx].outcomes[idx]
                .payload_and_rate
                .swap(
                    (old_payload_and_rate & 0xffFFffFF_00000000) | (to_fixed(new_rate) as u64),
                    Ordering::SeqCst,
                );
        }
    }

    pub fn extract<Random: Rng>(&self, rng: &mut Random) -> (ExtractStats, u32) {
        let u = rng.gen::<f32>();
        let mut rand = u * from_fixed(self.total_rate.load(Ordering::SeqCst));
        let mut iterations = 0;
        for (idx, g) in self.groups.iter().enumerate() {
            let sum_rate = from_fixed(self.sum_rates[idx].load(Ordering::SeqCst));
            if sum_rate > rand {
                let mut r = g.extract(rng);
                r.0.group_iterations = Some(iterations);
                return r;
            }

            iterations += 1;

            rand = rand - sum_rate;
        }

        panic!("Shouldn't be able to reach here, algorithm invariant breached");
    }
}

pub fn to_fixed(v: f32) -> u32 {
    let int = (v.floor() as u32) << 8u64;
    let frac = ((v - v.floor()) * 255.0) as u32;

    int + frac
}

pub fn from_fixed(v: u32) -> f32 {
    let nominator = (v >> 8u64) as f32;
    let denominator = (v & 0xffu32) as f32 / 255.0 as f32;

    nominator + denominator
}

#[derive(Clone)]
pub struct AliasMethod {
    alias: Vec<u32>,
    probability: Vec<f32>,
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
    } /*

    pub fn find_index(&self, u0: f32) -> usize {
        let u1 = ((b - a + 1) * u0) - ((b - a + 1.0) * u0).floor();
        self.find_index(u0, u1)
    }*/
}
