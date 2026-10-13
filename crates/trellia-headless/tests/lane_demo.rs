//! M5 验收：`lane_demo` 垂直切片的无窗口集成测试（SPEC.md 13 M5，UC-13、UC-23、UC-53、UC-74）。
//!
//! 运行：`cargo test -p trellia-headless --test lane_demo`
//!
//! golden 文件 `tests/golden/lane_demo_seed42.snap` 只在人工核对后以
//! `TRELLIA_BLESS=1 cargo test -p trellia-headless --test lane_demo lane_demo_determinism` 生成；
//! 之后任何导致其变化的改动都必须说明（AGENT.md 7）。

use lane_demo::rules::{Turret, Walker, WalkerState};
use lane_demo::{DEFAULT_SEED, build_app, cell_click_pos, register_snapshots};
use trellia_audio::AudioRequest;
use trellia_core::Key;
use trellia_headless::HeadlessRunner;
use trellia_scene::SceneStack;

/// 第 1 tick 点击格子 0、第 400 tick 点击格子 1。
const SCRIPT: &[(u32, u8)] = &[(1, 0), (400, 1)];

fn new_runner(seed: u64) -> HeadlessRunner {
    let mut runner = HeadlessRunner::new(build_app(seed));
    register_snapshots(&mut runner);
    runner
}

/// 推进到第 `tick` 帧（每帧恰好 1 tick），在脚本指定的 tick 点击格子。
fn step(runner: &mut HeadlessRunner, tick: u32, script: &[(u32, u8)]) {
    match script.iter().find(|(t, _)| *t == tick) {
        Some((_, cell)) => runner.click(cell_click_pos(*cell)),
        None => {
            runner.frame();
        }
    }
}

fn top(runner: &HeadlessRunner) -> Option<&'static str> {
    runner
        .app()
        .world()
        .resources
        .expect::<SceneStack>()
        .top_name()
}

/// 运行脚本 `ticks` 个 tick，返回首次进入 `scene` 的 tick。
fn run_until(
    runner: &mut HeadlessRunner,
    ticks: u32,
    script: &[(u32, u8)],
    scene: &str,
) -> Option<u32> {
    let mut reached = None;
    for t in 1..=ticks {
        step(runner, t, script);
        if reached.is_none() && top(runner) == Some(scene) {
            reached = Some(t);
        }
    }
    reached
}

fn golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/lane_demo_seed42.snap")
}

#[test]
fn lane_demo_determinism() {
    let snapshot = |_: ()| {
        let mut runner = new_runner(DEFAULT_SEED);
        for t in 1..=4000 {
            step(&mut runner, t, SCRIPT);
        }
        runner.snapshot()
    };
    let a = snapshot(());
    let b = snapshot(());
    assert_eq!(a, b, "两次运行快照必须逐字节相同");

    let path = golden_path();
    if std::env::var_os("TRELLIA_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("golden 目录")).expect("创建 golden 目录");
        std::fs::write(&path, &a).expect("写入 golden");
        return;
    }
    let golden = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "缺少 golden 文件 {}（{e}）。人工核对后以 TRELLIA_BLESS=1 生成",
            path.display()
        )
    });
    assert!(
        a == golden,
        "快照与 golden 不一致：{}（变化必须说明，见 AGENT.md 7）",
        path.display()
    );
}

#[test]
fn lane_demo_win_with_two_turrets() {
    let mut runner = new_runner(DEFAULT_SEED);
    let won = run_until(&mut runner, 4000, SCRIPT, "Victory");
    assert!(
        won.is_some(),
        "4000 tick 内应进入 Victory，实际栈顶 {:?}",
        top(&runner)
    );
}

#[test]
fn lane_demo_lose_without_turrets() {
    let mut runner = new_runner(DEFAULT_SEED);
    let lost = run_until(&mut runner, 2500, &[], "GameOver");
    assert!(
        lost.is_some(),
        "2500 tick 内应进入 GameOver，实际栈顶 {:?}",
        top(&runner)
    );
}

#[test]
fn lane_demo_bite_uses_event_frame() {
    let mut runner = new_runner(DEFAULT_SEED);
    let script = [(1, 4)];
    let turret_hp = |r: &HeadlessRunner| {
        r.app()
            .world()
            .query::<&Turret>()
            .iter()
            .map(|t| t.hp)
            .next()
    };
    let mut arrival = None;
    let mut first_damage = None;
    for t in 1..=1500 {
        step(&mut runner, t, &script);
        // 让第一个行者足够耐打，保证它能走到炮塔前
        for w in runner.app_mut().world_mut().query_mut::<&mut Walker>() {
            if w.hp == lane_demo::rules::WALKER_HP {
                w.hp = 250;
            }
        }
        let biting = runner
            .app()
            .world()
            .query::<&Walker>()
            .iter()
            .any(|w| matches!(w.state, WalkerState::Biting(_)));
        if biting && arrival.is_none() {
            arrival = Some(t);
            assert_eq!(turret_hp(&runner), Some(3), "到达当 tick 不扣血");
        }
        if arrival.is_some() && first_damage.is_none() && turret_hp(&runner) == Some(2) {
            first_damage = Some(t);
        }
    }
    let arrival = arrival.expect("行者应到达炮塔");
    let first_damage = first_damage.expect("炮塔应被啃食扣血");
    // walker_bite：bite_0 持续 20 tick，进入 bite_1（bite_hit 事件帧）时扣血
    assert_eq!(
        first_damage,
        arrival + 20,
        "扣血发生在 bite_hit 帧对应的 tick"
    );
}

#[test]
fn lane_demo_audio() {
    let mut runner = new_runner(DEFAULT_SEED);
    for t in 1..=4000 {
        step(&mut runner, t, SCRIPT);
    }
    let blip = runner
        .app()
        .world()
        .resources
        .expect::<lane_demo::rules::LaneAssets>()
        .blip;
    let log = runner.take_audio_log();
    let music = log
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlayMusic { .. }))
        .count();
    let blips = log
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlaySfx { clip, .. } if *clip == blip))
        .count();
    assert_eq!(music, 1);
    assert!(blips >= 10, "发射音效 {blips} 次，应 ≥ 10");
}

#[test]
fn lane_demo_pause_and_restart() {
    let mut runner = new_runner(DEFAULT_SEED);
    runner.frames(10);
    runner.press_key(Key::Escape);
    runner.frame();
    assert_eq!(top(&runner), Some("Paused"));
    let tick = |r: &HeadlessRunner| lane_demo::rules::lane(r.app().world()).map(|(_, s)| s.tick);
    let paused_at = tick(&runner);
    runner.frames(30);
    assert_eq!(tick(&runner), paused_at, "暂停时对局不推进");
    runner.press_key(Key::Space);
    runner.frames(2);
    assert_eq!(top(&runner), Some("Battle"));

    assert!(run_until(&mut runner, 2500, &[], "GameOver").is_some());
    runner.press_key(Key::Enter);
    runner.frames(2);
    assert_eq!(top(&runner), Some("Battle"));
    let stack = runner.app().world().resources.expect::<SceneStack>();
    assert_eq!(stack.depth(), 1);
    assert!(tick(&runner).is_some_and(|t| t < 5), "重开后对局从头开始");
}
