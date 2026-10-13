//! 运行：`cargo run --example core_counter`

fn main() {
    let (counter, snapshot) = core_counter::run(42, 600);
    println!(
        "core_counter: seed=42 ticks={} pulses={} rng_sum={} snapshot_bytes={}",
        counter.ticks,
        counter.pulses,
        counter.rng_sum,
        snapshot.len()
    );
}
