extern crate rand;
extern crate stats;
extern crate stochastic_data_structures;

use rand::*;
use stats::*;
use stochastic_data_structures::*;

fn main() {
    let max_rate = 10.0 * 2.0;
    let mut rj = CompositeRejectionMethod::<u32>::new(max_rate, 2.0);
    let mut rj = AtomicCompositeRejectionMethod::new(max_rate, 2.0, 20);
    //let mut rj = RejectionMethod::new(max_rate);

    rj.add(1.0, 0);
    rj.add(1.0, 1);
    let update_me = rj.add(1.0, 2);
    rj.add(1.0, 3);
    rj.add(1.0, 4);
    rj.add(1.0, 5);
    rj.add(1.0, 6);
    println!("{:#?}", rj);

    println!("{:?}", update_me);
    rj.update(update_me, 2.0);

    println!("{:#?}", rj);

    //rj.update(update_2, 1000.0);

    // println!("{:#?}", rj);

    let mut rng = thread_rng();

    let mut max_loop = 0;
    let mut max_group = 0;

    let mut list = [0.0; 7];
    let iter_count = 100000;

    let mut counts = vec![];

    for _x in 0..iter_count {
        let res = rj.extract(&mut rng);
        list[res.1 as usize] += 1.0 / iter_count as f32;
        max_loop = max_loop.max(res.0.loop_count);
        max_group = max_group.max(res.0.group_iterations.unwrap_or_default());
        counts.push(res.0);
    }

    println!("{:?}", list);

    let l_iter = counts.iter().map(|x| x.loop_count as f64);

    println!(
        "Loop count, max: {}, mean: {}, median: {}, stddev: {}, variance: {}",
        max_loop,
        mean(l_iter.clone()),
        median(l_iter.clone()).unwrap(),
        stddev(l_iter.clone()),
        variance(l_iter.clone())
    );

    let g_iter = counts
        .iter()
        .map(|x| x.group_iterations.unwrap_or_default() as f64);

    println!(
        "Group count, max: {}, mean: {}, median: {}, stddev: {}, variance: {}",
        max_group,
        mean(g_iter.clone()),
        median(g_iter.clone()).unwrap(),
        stddev(g_iter.clone()),
        variance(g_iter.clone())
    );

    println!("{}", from_fixed(to_fixed(1.0)));
}
