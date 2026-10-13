//! 运行：`cargo run --release --example sprites -- --count 1000`

fn main() {
    let mut count = 10;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--count" {
            match args.next().and_then(|n| n.parse().ok()) {
                Some(n) => count = n,
                None => {
                    eprintln!("用法: sprites [--count N]");
                    std::process::exit(2);
                }
            }
        }
    }
    let cfg = trellia::RunConfig {
        title: format!("sprites | {count} sprites"),
        ..trellia::RunConfig::default()
    };
    if let Err(e) = trellia::run_egui(sprites::build_app(count), cfg) {
        eprintln!("sprites: {e:#}");
        std::process::exit(1);
    }
}
