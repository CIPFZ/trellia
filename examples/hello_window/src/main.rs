//! 运行：`cargo run --example hello_window`

fn main() {
    let cfg = trellia::RunConfig {
        title: "hello_window".into(),
        ..trellia::RunConfig::default()
    };
    if let Err(e) = trellia::run_egui(hello_window::build_app(), cfg) {
        eprintln!("hello_window: {e:#}");
        std::process::exit(1);
    }
}
