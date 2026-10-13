//! 场景：战斗、暂停、结算覆盖层（UC-60、UC-61、UC-62）。

use trellia::egui;
use trellia::prelude::*;

use crate::render;
use crate::rules::{
    self, LaneAssets, LaneState, Outcome, PendingPlacements, Projectile, Turret, Walker,
};

/// 战斗场景：进入时以显式种子初始化对局（UC-13、UC-14），退出时清理全部对局实体。
pub struct BattleScene {
    /// 本局随机种子。
    pub seed: u64,
}

impl Scene for BattleScene {
    fn name(&self) -> &'static str {
        "Battle"
    }

    fn on_enter(&mut self, world: &mut World) {
        let mut rng = GameRng::from_seed(self.seed);
        let state = LaneState::new(&mut rng);
        world.resources.insert(rng);
        world.resources.insert(PendingPlacements::default());
        world.spawn((state,));
        if let Some(music) = world.resources.get::<LaneAssets>().map(|a| a.music)
            && let Some(queue) = world.resources.get_mut::<AudioQueue>()
        {
            queue.play_music(music, true);
        }
    }

    fn on_exit(&mut self, world: &mut World) {
        let mut doomed: Vec<Entity> = Vec::new();
        doomed.extend(world.query::<(Entity, &LaneState)>().iter().map(|(e, _)| e));
        doomed.extend(world.query::<(Entity, &Turret)>().iter().map(|(e, _)| e));
        doomed.extend(world.query::<(Entity, &Walker)>().iter().map(|(e, _)| e));
        doomed.extend(
            world
                .query::<(Entity, &Projectile)>()
                .iter()
                .map(|(e, _)| e),
        );
        for e in doomed {
            let _ = world.despawn(e);
        }
        if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
            queue.stop_music();
        }
    }

    fn fixed_update(&mut self, world: &mut World) {
        let Some(outcome) = rules::battle_tick(world) else {
            return;
        };
        world
            .resources
            .expect_mut::<SceneCommands>()
            .push(OutcomeScene::new(outcome == Outcome::Won, self.seed));
    }

    /// 输入每帧处理一次：点击转为放置意图，下一 tick 应用（UC-40、UC-42）。
    fn update(&mut self, world: &mut World) {
        let input = world.resources.expect::<Input>();
        let pause = input.key_just_pressed(Key::Escape);
        let cell = input
            .mouse_just_pressed(MouseButton::Left)
            .then_some(input.mouse_pos)
            .flatten()
            .and_then(rules::cell_at);
        if let Some(cell) = cell {
            world
                .resources
                .expect_mut::<PendingPlacements>()
                .0
                .push(cell);
        }
        if pause {
            world
                .resources
                .expect_mut::<SceneCommands>()
                .push(PauseScene);
        }
    }

    fn draw(&mut self, world: &mut World) {
        render::draw_battle(world);
    }
}

/// 暂停覆盖层：模拟停止（`Time.paused`），下方画面冻结但可见；Esc / 空格继续（UC-11、UC-41、UC-61）。
pub struct PauseScene;

impl Scene for PauseScene {
    fn name(&self) -> &'static str {
        "Paused"
    }

    fn on_enter(&mut self, world: &mut World) {
        world.resources.expect_mut::<Time>().paused = true;
    }

    fn on_exit(&mut self, world: &mut World) {
        world.resources.expect_mut::<Time>().paused = false;
    }

    fn update(&mut self, world: &mut World) {
        let input = world.resources.expect::<Input>();
        if input.key_just_pressed(Key::Escape) || input.key_just_pressed(Key::Space) {
            world.resources.expect_mut::<SceneCommands>().pop();
        }
    }

    fn draw(&mut self, world: &mut World) {
        render::overlay(world, "PAUSED", "Esc / Space to continue");
    }
}

/// 结算覆盖层：失败为 `GameOver`，胜利为 `Victory`；“Restart” 按钮或回车重开（UC-60、UC-61）。
pub struct OutcomeScene {
    won: bool,
    seed: u64,
    restarting: bool,
}

impl OutcomeScene {
    /// 创建结算场景；`seed` 为重开时使用的种子。
    #[must_use]
    pub fn new(won: bool, seed: u64) -> Self {
        Self {
            won,
            seed,
            restarting: false,
        }
    }

    /// 弹出自身并以新的战斗场景替换旧战斗（同一帧内只发出一次）。
    fn restart(&mut self, world: &mut World) {
        if std::mem::replace(&mut self.restarting, true) {
            return;
        }
        let commands = world.resources.expect_mut::<SceneCommands>();
        commands.pop();
        commands.replace(BattleScene { seed: self.seed });
    }
}

impl Scene for OutcomeScene {
    fn name(&self) -> &'static str {
        if self.won { "Victory" } else { "GameOver" }
    }

    fn update(&mut self, world: &mut World) {
        if world
            .resources
            .expect::<Input>()
            .key_just_pressed(Key::Enter)
        {
            self.restart(world);
        }
    }

    fn draw(&mut self, world: &mut World) {
        let title = if self.won { "VICTORY" } else { "GAME OVER" };
        render::overlay(world, title, "Restart (Enter)");
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let mut clicked = false;
        egui::Area::new(egui::Id::new("lane_demo_restart"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 80.0))
            .show(ctx, |ui| {
                let button = egui::Button::new(egui::RichText::new("Restart").size(28.0));
                clicked = ui.add(button).clicked();
            });
        if clicked {
            self.restart(world);
        }
    }
}
