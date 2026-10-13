//! 车道塔防规则（SPEC.md 13 M5）：常量、组件与每 tick 的模拟逻辑。
//!
//! 所有逻辑只在 `BattleScene::fixed_update` 中运行，遵守 SPEC.md 12 确定性规则：
//! 时长用整数 tick、随机只来自 `GameRng`、无 HashMap 迭代、浮点只做加减与阈值比较。

use trellia::headless::Snapshot;
use trellia::prelude::*;

/// 车道中心 y。
pub const LANE_Y: f32 = 400.0;
/// 车道高度（点击判定与绘制）。
pub const LANE_HEIGHT: f32 = 100.0;
/// 格子数。
pub const CELL_COUNT: u8 = 9;
/// 格子宽度。
pub const CELL_WIDTH: f32 = 100.0;
/// 第 0 格左边界 x。
pub const LANE_START_X: f32 = 100.0;
/// 炮塔花费。
pub const TURRET_COST: u32 = 50;
/// 炮塔生命。
pub const TURRET_HP: u8 = 3;
/// 炮塔射击周期（tick）。
pub const TURRET_FIRE_TICKS: u32 = 90;
/// 弹丸速度（逻辑像素 / tick）。
pub const PROJECTILE_SPEED: f32 = 6.0;
/// 弹丸伤害。
pub const PROJECTILE_DAMAGE: u8 = 1;
/// 弹丸销毁边界 x。
pub const PROJECTILE_MAX_X: f32 = 1280.0;
/// 行者生成 x。
pub const WALKER_SPAWN_X: f32 = 1300.0;
/// 行者速度（逻辑像素 / tick）。
pub const WALKER_SPEED: f32 = 0.6;
/// 行者生命。
pub const WALKER_HP: u8 = 5;
/// 行者碰撞区间半宽。
pub const WALKER_HALF_WIDTH: f32 = 25.0;
/// 行者总数。
pub const WALKER_COUNT: usize = 5;
/// 第 1 个行者的基准生成 tick。
pub const FIRST_SPAWN_TICK: u32 = 120;
/// 生成间隔（tick）。
pub const SPAWN_INTERVAL_TICKS: u32 = 600;
/// 生成抖动上限（不含），`GameRng.gen_range(0..30)`。
pub const SPAWN_JITTER_TICKS: u32 = 30;
/// 初始资源。
pub const START_MONEY: u32 = 150;
/// 每次收入。
pub const INCOME: u32 = 25;
/// 收入周期（tick）。
pub const INCOME_TICKS: u32 = 300;
/// 任一行者 x 小于此值即失败。
pub const LOSE_X: f32 = 60.0;
/// 啃食动画的伤害事件名。
pub const BITE_EVENT: &str = "bite_hit";

/// 第 `cell` 格的矩形。
#[must_use]
pub fn cell_rect(cell: u8) -> Rect {
    Rect::from_min_size(
        Vec2::new(
            LANE_START_X + f32::from(cell) * CELL_WIDTH,
            LANE_Y - LANE_HEIGHT / 2.0,
        ),
        Vec2::new(CELL_WIDTH, LANE_HEIGHT),
    )
}

/// 第 `cell` 格的中心。
#[must_use]
pub fn cell_center(cell: u8) -> Vec2 {
    cell_rect(cell).center()
}

/// 逻辑坐标所在格子。
#[must_use]
pub fn cell_at(pos: Vec2) -> Option<u8> {
    (0..CELL_COUNT).find(|&c| cell_rect(c).contains(pos))
}

/// 占位渲染组件（SPEC.md 10.3）：由 `render` 转为 `Rect + Text`。对应 UC-27。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placeholder {
    /// 尺寸（以 `Transform2D.pos` 为中心）。
    pub size: Vec2,
    /// 颜色。
    pub color: Color,
    /// 标签文字。
    pub label: &'static str,
}

/// 炮塔。对应 UC-01、UC-03、UC-12。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turret {
    /// 所在格子。
    pub cell: u8,
    /// 剩余生命。
    pub hp: u8,
    /// 射击周期计时器。
    pub fire: Timer,
}

/// 行者状态。对应 UC-03、UC-23。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkerState {
    /// 向左行走。
    Walking,
    /// 停下啃食目标炮塔。
    Biting(Entity),
}

/// 行者。对应 UC-03、UC-23。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Walker {
    /// 剩余生命。
    pub hp: u8,
    /// 当前状态。
    pub state: WalkerState,
}

/// 弹丸标记。对应 UC-03、UC-04。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projectile;

/// 对局结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 进行中。
    Playing,
    /// 失败。
    Lost,
    /// 胜利。
    Won,
}

/// 对局状态，挂在唯一的“车道实体”上，以便参与确定性快照。对应 UC-12、UC-13、UC-14。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneState {
    /// 本局已执行的 tick 数。
    pub tick: u32,
    /// 当前资源。
    pub money: u32,
    /// 收入计时器。
    pub income: Timer,
    /// 每个行者的生成 tick（基准 + 抖动）。
    pub schedule: [u32; WALKER_COUNT],
    /// 已生成的行者数。
    pub spawned: u8,
    /// 已击杀的行者数。
    pub killed: u8,
    /// 发射的弹丸数。
    pub shots: u32,
    /// 命中次数。
    pub hits: u32,
    /// 对局结果。
    pub outcome: Outcome,
}

impl LaneState {
    /// 以种子化随机数生成本局的生成时刻表。
    #[must_use]
    pub fn new(rng: &mut GameRng) -> Self {
        let mut schedule = [0; WALKER_COUNT];
        for (i, slot) in (0_u32..).zip(schedule.iter_mut()) {
            *slot =
                FIRST_SPAWN_TICK + SPAWN_INTERVAL_TICKS * i + rng.gen_range(0..SPAWN_JITTER_TICKS);
        }
        Self {
            tick: 0,
            money: START_MONEY,
            income: Timer::repeating(INCOME_TICKS),
            schedule,
            spawned: 0,
            killed: 0,
            shots: 0,
            hits: 0,
            outcome: Outcome::Playing,
        }
    }
}

/// 资源：本帧点击产生、下一 tick 应用的放置意图（格子号）。对应 UC-40、UC-42。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingPlacements(pub Vec<u8>);

/// 资源：本示例用到的资产句柄。对应 UC-30、UC-31。
#[derive(Clone, Copy, Debug)]
pub struct LaneAssets {
    /// 行者图集。
    pub atlas: Handle<Atlas>,
    /// 行走片段。
    pub walk: Handle<AnimationClip>,
    /// 啃食片段（`bite_hit` 事件帧）。
    pub bite: Handle<AnimationClip>,
    /// 发射音效。
    pub blip: Handle<AudioBytes>,
    /// 命中音效。
    pub hit: Handle<AudioBytes>,
    /// 循环 BGM。
    pub music: Handle<AudioBytes>,
}

/// 取车道实体与其状态副本。
#[must_use]
pub fn lane(world: &World) -> Option<(Entity, LaneState)> {
    world
        .query::<(Entity, &LaneState)>()
        .iter()
        .next()
        .map(|(e, s)| (e, *s))
}

fn play(world: &mut World, clip: impl Fn(&LaneAssets) -> Handle<AudioBytes>) {
    let Some(assets) = world.resources.get::<LaneAssets>().copied() else {
        return;
    };
    if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
        queue.play_sfx(clip(&assets));
    }
}

/// 推进一个 tick 的对局逻辑。由 `BattleScene::fixed_update` 调用。
///
/// 返回本 tick 刚刚确定的结果（只在结果产生的那个 tick 返回 `Some`）。
pub fn battle_tick(world: &mut World) -> Option<Outcome> {
    let (lane_entity, mut state) = lane(world)?;
    if state.outcome != Outcome::Playing {
        return None;
    }
    state.tick += 1;
    if state.income.tick() {
        state.money += INCOME;
    }
    place_turrets(world, &mut state);
    spawn_walkers(world, &mut state);
    apply_bites(world);
    fire_turrets(world, &mut state);
    move_projectiles(world, &mut state);
    move_walkers(world);
    check_outcome(world, &mut state);
    if let Some(mut s) = world.get_mut::<LaneState>(lane_entity) {
        *s = state;
    }
    (state.outcome != Outcome::Playing).then_some(state.outcome)
}

fn place_turrets(world: &mut World, state: &mut LaneState) {
    let pending = std::mem::take(&mut world.resources.expect_mut::<PendingPlacements>().0);
    for cell in pending {
        let occupied = world
            .query::<&Turret>()
            .iter()
            .any(|t| t.cell == cell && t.hp > 0);
        if occupied || state.money < TURRET_COST {
            continue;
        }
        state.money -= TURRET_COST;
        world.spawn((
            Transform2D::new(cell_center(cell)),
            Turret {
                cell,
                hp: TURRET_HP,
                fire: Timer::repeating(TURRET_FIRE_TICKS),
            },
            Placeholder {
                size: Vec2::new(70.0, 70.0),
                color: Color::from_hex(0x27_AE60),
                label: "T",
            },
        ));
    }
}

fn spawn_walkers(world: &mut World, state: &mut LaneState) {
    let Some(assets) = world.resources.get::<LaneAssets>().copied() else {
        return;
    };
    while usize::from(state.spawned) < WALKER_COUNT
        && state.tick >= state.schedule[usize::from(state.spawned)]
    {
        state.spawned += 1;
        let mut sprite = Sprite::new(assets.atlas);
        sprite.visible = false; // 占位渲染；Sprite 只为驱动动画事件
        world.spawn((
            Transform2D::new(Vec2::new(WALKER_SPAWN_X, LANE_Y)),
            Walker {
                hp: WALKER_HP,
                state: WalkerState::Walking,
            },
            sprite,
            Animator::new(assets.walk),
            Placeholder {
                size: Vec2::new(50.0, 80.0),
                color: Color::from_hex(0x8E_44AD),
                label: "W",
            },
        ));
    }
}

/// 动画事件帧 `bite_hit` 才真正扣血（UC-23）。
fn apply_bites(world: &mut World) {
    let events: Vec<AnimationEvent> = world.events.get_mut::<AnimationEvent>().drain().collect();
    for ev in events {
        if ev.event != BITE_EVENT {
            continue;
        }
        let Some(WalkerState::Biting(target)) = world.get::<Walker>(ev.entity).map(|w| w.state)
        else {
            continue;
        };
        let destroyed = match world.get_mut::<Turret>(target) {
            Some(mut turret) if turret.hp > 0 => {
                turret.hp -= 1;
                turret.hp == 0
            }
            _ => false,
        };
        if destroyed {
            world.commands.despawn(target);
        }
    }
}

fn fire_turrets(world: &mut World, state: &mut LaneState) {
    let walker_xs: Vec<f32> = world
        .query::<(&Transform2D, &Walker)>()
        .iter()
        .filter(|(_, w)| w.hp > 0)
        .map(|(t, _)| t.pos.x)
        .collect();
    let mut shots = Vec::new();
    for (t, turret) in world.entities.query_mut::<(&Transform2D, &mut Turret)>() {
        if turret.fire.tick() && turret.hp > 0 && walker_xs.iter().any(|&x| x > t.pos.x) {
            shots.push(t.pos);
        }
    }
    for pos in shots {
        world.spawn((Transform2D::new(pos), Projectile));
        state.shots += 1;
        play(world, |a| a.blip);
    }
}

fn move_projectiles(world: &mut World, state: &mut LaneState) {
    let mut walkers: Vec<(Entity, f32, u8)> = world
        .query::<(Entity, &Transform2D, &Walker)>()
        .iter()
        .filter(|(_, _, w)| w.hp > 0)
        .map(|(e, t, w)| (e, t.pos.x, w.hp))
        .collect();
    let mut hits = 0;
    for (e, t, _) in world
        .entities
        .query_mut::<(Entity, &mut Transform2D, &Projectile)>()
    {
        t.pos.x += PROJECTILE_SPEED;
        if t.pos.x > PROJECTILE_MAX_X {
            world.commands.despawn(e);
            continue;
        }
        let x = t.pos.x;
        let target = walkers
            .iter_mut()
            .filter(|(_, wx, hp)| *hp > 0 && (wx - x).abs() <= WALKER_HALF_WIDTH)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((_, _, hp)) = target {
            *hp = hp.saturating_sub(PROJECTILE_DAMAGE);
            hits += 1;
            world.commands.despawn(e);
        }
    }
    for (e, _, hp) in walkers {
        if let Some(mut w) = world.get_mut::<Walker>(e) {
            w.hp = hp;
        }
        if hp == 0 {
            world.commands.despawn(e);
            state.killed += 1;
        }
    }
    state.hits += hits;
    for _ in 0..hits {
        play(world, |a| a.hit);
    }
}

fn move_walkers(world: &mut World) {
    let Some(assets) = world.resources.get::<LaneAssets>().copied() else {
        return;
    };
    let turrets: Vec<(Entity, u8)> = world
        .query::<(Entity, &Turret)>()
        .iter()
        .filter(|(_, t)| t.hp > 0)
        .map(|(e, t)| (e, t.cell))
        .collect();
    for (t, walker, anim) in world
        .entities
        .query_mut::<(&mut Transform2D, &mut Walker, &mut Animator)>()
    {
        if walker.hp == 0 {
            continue;
        }
        if let WalkerState::Biting(target) = walker.state {
            if turrets.iter().any(|(e, _)| *e == target) {
                continue;
            }
            walker.state = WalkerState::Walking;
            anim.play(assets.walk);
        }
        let (left, right) = (t.pos.x - WALKER_HALF_WIDTH, t.pos.x + WALKER_HALF_WIDTH);
        let contact = turrets
            .iter()
            .filter(|(_, cell)| {
                let r = cell_rect(*cell);
                r.min.x <= right && left <= r.max.x
            })
            .max_by_key(|(_, cell)| *cell);
        if let Some((target, _)) = contact {
            walker.state = WalkerState::Biting(*target);
            anim.play(assets.bite);
        } else {
            t.pos.x -= WALKER_SPEED;
        }
    }
}

fn check_outcome(world: &mut World, state: &mut LaneState) {
    let lost = world
        .query::<(&Transform2D, &Walker)>()
        .iter()
        .any(|(t, w)| w.hp > 0 && t.pos.x < LOSE_X);
    if lost {
        state.outcome = Outcome::Lost;
    } else if usize::from(state.killed) == WALKER_COUNT {
        state.outcome = Outcome::Won;
    }
}

impl Snapshot for Turret {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend([self.cell, self.hp]);
        out.extend(self.fire.remaining.to_le_bytes());
    }
}

impl Snapshot for Walker {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.hp);
        match self.state {
            WalkerState::Walking => out.extend(0_u64.to_le_bytes()),
            WalkerState::Biting(target) => out.extend(target.to_bits().get().to_le_bytes()),
        }
    }
}

impl Snapshot for Projectile {
    fn write(&self, _out: &mut Vec<u8>) {}
}

impl Snapshot for LaneState {
    fn write(&self, out: &mut Vec<u8>) {
        for v in [
            self.tick,
            self.money,
            self.income.remaining,
            self.shots,
            self.hits,
        ] {
            out.extend(v.to_le_bytes());
        }
        for v in self.schedule {
            out.extend(v.to_le_bytes());
        }
        let outcome = match self.outcome {
            Outcome::Playing => 0,
            Outcome::Lost => 1,
            Outcome::Won => 2,
        };
        out.extend([self.spawned, self.killed, outcome]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_cover_lane() {
        assert_eq!(cell_center(0), Vec2::new(150.0, 400.0));
        assert_eq!(cell_at(Vec2::new(150.0, 400.0)), Some(0));
        assert_eq!(cell_at(Vec2::new(999.0, 449.0)), Some(8));
        assert_eq!(cell_at(Vec2::new(1000.0, 400.0)), None);
        assert_eq!(cell_at(Vec2::new(150.0, 300.0)), None);
    }

    #[test]
    fn schedule_has_jitter_within_bounds() {
        let state = LaneState::new(&mut GameRng::from_seed(42));
        for (i, t) in (0_u32..).zip(state.schedule) {
            let base = FIRST_SPAWN_TICK + SPAWN_INTERVAL_TICKS * i;
            assert!((base..base + SPAWN_JITTER_TICKS).contains(&t));
        }
        assert_eq!(state, LaneState::new(&mut GameRng::from_seed(42)));
    }
}
