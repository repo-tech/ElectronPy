use std::env;

fn compute(limit: i64, bias: i64) -> i64 {
    let mut total = 0_i64;
    for i in 0..=limit {
        total += i + bias;
    }
    total
}

fn main() {
    let limit: i64 = env::var("BENCH_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000_000);
    let bias: i64 = env::var("BENCH_BIAS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    println!("{}", compute(limit, bias));
}
