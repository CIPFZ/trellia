//! 运行：`cargo run --release --example lane_demo`

fn main() {
    let cfg = trellia::RunConfig {
        title: "lane_demo".into(),
        ..trellia::RunConfig::default()
    };
    if let Err(e) = trellia::run_egui(lane_demo::build_app(lane_demo::DEFAULT_SEED), cfg) {
        eprintln!("lane_demo: {e:#}");
        std::process::exit(1);
    }
}
