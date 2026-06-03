pub const MAP_SCALE: f64 = 2.2;
pub const BASE_SPEED: f64 = 310.0;
pub const SPEED_ALPHA: f64 = 0.35;
/// 大泡泡最低巡航速度不低于初始速度的该比例
pub const MIN_SPEED_RATIO: f64 = 0.38;
/// 质量越大，转向加速越慢；质量越小，加速越明显
pub const DRIVE_ACCEL_BASE: f64 = 1180.0;
pub const DRIVE_ACCEL_ALPHA: f64 = 0.88;
/// 排空推力系数：推力 ∝ 本帧排出能量 / 质量
pub const EXPEL_THRUST_COEFF: f64 = 5200.0;
/// 排空时允许的最高速度倍率（相对巡航速度）
pub const EXPEL_MAX_SPEED_MULT: f64 = 2.15;
/// 排空时朝目标速度加速的倍率（更快进入排空速度）
pub const EXPEL_DRIVE_ACCEL_MULT: f64 = 2.8;
pub const EXPEL_RATE_BASE: f64 = 28.0;
/// 大泡泡排空速率随半径的指数（越大排出越多）
pub const EXPEL_RATE_SIZE_POWER: f64 = 1.38;
pub const INITIAL_RADIUS: f64 = 12.0;
pub const UNIT_RADIUS: f64 = 5.0;
pub const UNIT_ENERGY: f64 = std::f64::consts::PI * UNIT_RADIUS * UNIT_RADIUS;
/// 吃掉单位泡泡时，实际获得的能量倍率
pub const UNIT_ENERGY_GAIN: f64 = 2.8;
pub const UNIT_SPAWN_INTERVAL: f64 = 0.4;
/// 地图上富集区数量（其余区域以分散泡泡为主）
pub const UNIT_CLUSTER_COUNT: usize = 3;
pub const UNIT_CLUSTER_INITIAL: usize = 7;
/// 开局分散在地图各处的单位泡泡数量
pub const UNIT_SCATTER_INITIAL: usize = 38;
pub const UNIT_CLUSTER_SPAWN_BATCH: usize = 2;
pub const UNIT_SCATTER_SPAWN_BATCH: usize = 1;
/// 持续刷新时，分散刷新的概率
pub const UNIT_SCATTER_SPAWN_CHANCE: f64 = 0.68;
/// 持续刷新时，新建富集区的概率（在非分散刷新时）
pub const UNIT_NEW_CLUSTER_CHANCE: f64 = 0.15;
pub const UNIT_CLUSTER_SPREAD: f64 = 45.0;
pub const UNIT_CLUSTER_TIGHT_SPREAD: f64 = 24.0;
pub const EAT_RATIO: f64 = 1.05;
/// AI 吃掉玩家所需的最小半径倍率（须比玩家大 5%）
pub const AI_EAT_PLAYER_RATIO: f64 = 1.05;
pub const COVER_DEATH_RATIO: f64 = 2.0 / 3.0;
pub const MIN_AI: usize = 8;
pub const MAX_AI: usize = 20;
pub const AI_DENSITY: f64 = 24000.0;
/// AI 刷新间隔随机范围（秒）
pub const AI_SPAWN_INTERVAL_MIN: f64 = 1.2;
pub const AI_SPAWN_INTERVAL_MAX: f64 = 4.8;
/// 低于此数量时会提高刷新概率，避免场上 AI 过少
pub const AI_SOFT_MIN: usize = 7;
/// 高于此数量时降低刷新概率，但不完全停止
pub const AI_SOFT_MAX: usize = 22;
/// 场上 AI 绝对上限（含自杀式 AI）
pub const AI_HARD_MAX: usize = 32;
/// 刷新时比玩家更大的概率（制造持续危机感）
pub const AI_SPAWN_LARGE_CHANCE: f64 = 0.14;
pub const AI_SPAWN_LARGE_MIN: f64 = 1.06;
pub const AI_SPAWN_LARGE_MAX: f64 = 1.42;
/// 刷新为“聪明 AI”的概率
pub const AI_SMART_CHANCE: f64 = 0.34;
/// 刷新为“围剿型 Hunter”（比玩家大、主动追玩家）的概率
pub const AI_HUNTER_SPAWN_CHANCE: f64 = 0.09;
/// 在玩家当前可视范围内刷新的概率
pub const AI_SPAWN_IN_VIEW_CHANCE: f64 = 0.2;
pub const AI_HUNTER_MIN: f64 = 1.12;
pub const AI_HUNTER_MAX: f64 = 1.5;
/// Hunter 追击玩家的持续时间（秒，随机区间）
pub const AI_HUNTER_CHASE_MIN: f64 = 4.0;
pub const AI_HUNTER_CHASE_MAX: f64 = 7.0;
/// 兴趣转移后，再次追击玩家的冷却（秒）
pub const AI_HUNTER_PLAYER_COOLDOWN_MIN: f64 = 10.0;
pub const AI_HUNTER_PLAYER_COOLDOWN_MAX: f64 = 16.0;
/// 追击同一目标（玩家或其它 AI）的感兴趣时间（秒）
pub const AI_INTEREST_MIN: f64 = 3.5;
pub const AI_INTEREST_MAX: f64 = 7.5;
/// 放弃追击后，专注觅食单位/尸体的时间（秒）
pub const AI_FORAGE_MIN: f64 = 4.5;
pub const AI_FORAGE_MAX: f64 = 9.0;
/// 已在玩家视野内时，朝视野中心靠拢的权重
pub const AI_VIEWPORT_PULL_INNER: f64 = 0.14;
/// 在视野外时，朝玩家视野移动的权重上限
pub const AI_VIEWPORT_PULL_OUTER: f64 = 0.42;
/// 贴边判定：距边界小于此值视为贴边
pub const AI_WALL_NEAR: f64 = 58.0;
/// 方向评分中贴边影响带宽度（在 AI_WALL_NEAR 基础上再延伸）
pub const AI_WALL_BAND_EXTRA: f64 = 38.0;
/// 朝墙移动时的最大方向惩罚
pub const AI_WALL_PENALTY_MAX: f64 = 9.5;
/// 贴边时朝地图内部移动的奖励
pub const AI_WALL_INWARD_BONUS: f64 = 5.8;
/// 贴边时沿墙滑行的额外惩罚
pub const AI_WALL_SLIDE_PENALTY: f64 = 5.5;
/// anti_wall_slide 最小/最大改向混合比
pub const AI_WALL_SLIDE_BLEND_MIN: f64 = 0.62;
pub const AI_WALL_SLIDE_BLEND_MAX: f64 = 0.96;
/// 贴边时额外偏向地图中心的评分权重
pub const AI_WALL_CENTER_BONUS: f64 = 0.42;
/// 沿墙判定：朝地图内部的移动分量低于此值视为贴边滑行
pub const AI_WALL_HUG_SLIDE_DOT: f64 = 0.58;
/// 持续贴边超过此秒数后必定触发离墙意图
pub const AI_WALL_HUG_FORCE_AFTER: f64 = 0.45;
/// 刚发现贴边时触发离墙意图的基础概率
pub const AI_WALL_DEPART_CHANCE_BASE: f64 = 0.90;
/// 长期贴边时离墙意图概率上限
pub const AI_WALL_DEPART_CHANCE_MAX: f64 = 0.99;
/// 离墙意图最短保持时间（秒）
pub const AI_WALL_DEPART_INTENT_MIN: f64 = 0.85;
/// 贴边时削弱视野拉拢，避免被拉向地图边缘
pub const AI_WALL_VIEWPORT_DAMP: f64 = 0.22;
/// 被更大泡泡围堵时排空逃离的概率
pub const AI_SURROUND_EXPEL_CHANCE: f64 = 0.72;
pub const AI_FLEE_EXPEL_CHANCE: f64 = 0.45;
pub const AI_SENSE_RADIUS: f64 = 300.0;
pub const AI_VISIBLE_RADIUS: f64 = 280.0;
pub const AI_SMART_SENSE_RADIUS: f64 = 340.0;
pub const AI_FLEE_RATIO: f64 = 1.1;
pub const AI_MISTAKE_RATE: f64 = 0.03;
pub const AI_SMART_MISTAKE_RATE: f64 = 0.01;
pub const AI_STEER_RATE: f64 = 4.0;
/// AI 每秒最大转向角（弧度），防止瞬间反向
pub const AI_MAX_TURN_RATE: f64 = 2.6;
/// 重新评估移动意图的间隔（秒）
pub const AI_INTENT_INTERVAL: f64 = 1.0;
pub const AI_SMART_INTENT_INTERVAL: f64 = 0.75;
/// 判断被其它 AI 围堵、暂不强制离边的检测半径
pub const AI_BOUNDARY_BLOCK_RADIUS: f64 = 130.0;
/// 偶尔原地打转的概率
pub const AI_SPIN_CHANCE: f64 = 0.05;
pub const AI_SPIN_MIN: f64 = 0.5;
pub const AI_SPIN_MAX: f64 = 1.1;
pub const AI_SPIN_TURN_RATE: f64 = 2.2;
pub const SUICIDE_SPAWN_CHANCE: f64 = 0.35;
pub const SUICIDE_RADIUS_MIN: f64 = 0.65;
pub const SUICIDE_RADIUS_MAX: f64 = 0.88;
pub const WIN_DIAMETER_RATIO: f64 = 2.0 / 3.0;
pub const PLAYER_COLOR: (u8, u8, u8) = (255, 110, 180);
pub const BG_COLOR: (u8, u8, u8) = (26, 26, 46);
pub const DEATH_DROP_FRACTION: f64 = 2.0 / 3.0;
pub const DEATH_DROP_MIN: usize = 6;
pub const DEATH_DROP_MAX: usize = 12;
/// 死亡尸体沿移动方向喷射的基础距离
pub const DEATH_SPRAY_BASE: f64 = 10.0;
/// 死亡尸体喷射距离随泡泡半径的倍率
pub const DEATH_SPRAY_RADIUS_SCALE: f64 = 1.4;
/// 喷射束横向散布基础宽度
pub const DEATH_SPRAY_LATERAL_BASE: f64 = 10.0;
/// 横向散布随泡泡半径的倍率
pub const DEATH_SPRAY_LATERAL_RADIUS_SCALE: f64 = 0.9;
/// 吞噬非单位泡泡时，吞噬者立即获得的能量比例
pub const EAT_ABSORB_FRACTION: f64 = 3.0 / 5.0;
/// 吞噬后残留为尸体泡泡的能量比例
pub const EAT_REMAIN_FRACTION: f64 = 2.0 / 5.0;
/// 尸体集群中心距吞噬者边缘的最小间距
pub const EAT_REMAIN_MIN_GAP: f64 = 14.0;
pub const EAT_REMAIN_FORWARD_SPREAD: f64 = 38.0;
pub const EAT_REMAIN_LATERAL_SPREAD: f64 = 26.0;
/// 掉落能量随机权重区间（用于生成大小不一的尸体/单位泡泡）
pub const DROP_ENERGY_WEIGHT_MIN: f64 = 0.22;
pub const DROP_ENERGY_WEIGHT_MAX: f64 = 2.6;
pub const MIN_DROP_ENERGY: f64 = UNIT_ENERGY * 0.12;
/// 单位泡泡渲染透明度（静止）
pub const STATIC_UNIT_ALPHA: f64 = 0.58;
/// 尸体泡泡渲染透明度（静止）
pub const REMAINS_ALPHA: f64 = 0.46;
/// AI 达到玩家最终大小时触发裂解的半径比例
pub const AI_FISSION_RADIUS_THRESHOLD: f64 = 0.995;
pub const AI_FISSION_MIN: usize = 3;
pub const AI_FISSION_MAX: usize = 6;
/// 裂解产出（子 AI + 尸体）保留的母体能量比例
pub const AI_FISSION_ENERGY_RETAIN: f64 = 0.75;
/// 裂解后子泡泡初始斥力速度
pub const AI_FISSION_REPULSE_BASE: f64 = 155.0;
pub const AI_FISSION_REPULSE_SCALE: f64 = 0.62;
/// 裂解后仍产生比玩家大的子 AI 的概率（保留压迫感）
pub const AI_FISSION_REMAIN_LARGE_CHANCE: f64 = 0.18;
/// 偏小裂解：子 AI 半径相对当前玩家的下限/上限
pub const AI_FISSION_SMALL_MIN_RATIO: f64 = 0.38;
pub const AI_FISSION_SMALL_MAX_RATIO: f64 = 0.90;
/// 偏大裂解：仍比玩家大的那个子 AI 半径倍率
pub const AI_FISSION_LARGE_MIN_RATIO: f64 = 1.06;
pub const AI_FISSION_LARGE_MAX_RATIO: f64 = 1.32;
