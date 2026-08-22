<!--
  超级马里奥复刻 —— 浏览器 Canvas 平台跳跃游戏

  玩法：
  - ←/→（或 A/D）：左右移动
  - ↑/W/Space（或路上按跳）：跳跃（可长按跳更高）
  - R：当前局暂停时重新开始；Enter：开始 / 重新开始
  - 顶 '?' 块出金币、顶 'B' 砖可顶碎；踩敌人消灭；收集金币加分
  - 到达终点旗杆即可通关，得分自动提交到高分榜
-->
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useAuthStore } from "@/stores/auth";
import { marioApi } from "@/api";
import { useWindowScale } from "@/mario/composables/useWindowScale";

// ============ 基础常量 ============
const TILE = 16; // 瓦片边长（像素）
const VIEW_W = 480; // 画布逻辑宽度
const VIEW_H = 320; // 画布逻辑高度
const ROWS = VIEW_H / TILE; // 20 行
const GRAV = 0.5; // 重力
const MAX_FALL = 10; // 最大下落速度

// ============ 瓦片说明 ============
// ' ' 空、'X' 实心地/砖、'B' 可顶碎砖、'Q' 问号块、'#' 已使用块
// 'P' 水管壁、'o' 悬浮金币、'K' 终点城堡、'H' 隐藏砖（不可见，顶到才出金币并显现）
type TileChar = " " | "X" | "B" | "Q" | "#" | "P" | "o" | "K" | "H";

const canvasRef = ref<HTMLCanvasElement | null>(null);
// 类型上视为已初始化（onMounted 统一赋值），运行时各绘制入口均以 `if (!ctx) return;` 兜底
let ctx!: CanvasRenderingContext2D;

const authStore = useAuthStore();

// 界面等比缩放：参考 fj200c_main 的 useWindowScale 机制，
// 以游戏画布 480×320 为设计稿，CSS transform: scale 放大居中并限制最大倍率
const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale({
  designWidth: VIEW_W,
  designHeight: VIEW_H,
});
const stageStyle = computed(() => ({
  width: DESIGN_W + "px",
  height: DESIGN_H + "px",
  transform: "scale(" + scale.value + ")",
}));

// ============ 游戏状态（非响应式，性能优先） ============
const game = {
  cols: 200,
  grid: [] as TileChar[][], // grid[row][col]
  mario: { x: 0, y: 0, w: 14, h: 20, vx: 0, vy: 0, onGround: false, facing: 1, anim: 0, size: 0 as 0 | 1, inv: 0, fire: 0 as 0 | 1 },
  enemies: [] as Enemy[],
  items: [] as Item[],
  fireballs: [] as Fireball[],
  particles: [] as Particle[],
  mushroomBlocks: new Set<number>(), // 产出“变大蘑菇”的问号块列号
  flowerBlocks: new Set<number>(), // 产出“火花花”的问号块列号
  coinBricks: new Set<number>(), // 顶出金币（而非碎裂）的砖块列号
  oneupBlocks: new Set<number>(), // 顶出 1UP 绿蘑菇的问号块列号
  state: "title" as "title" | "play" | "dead" | "clear",
  score: 0,
  coins: 0,
  lives: 3,
  time: 200,
  level: 1,
  camera: 0,
  frame: 0,
  timer: 0,
  fireCd: 0, // 火球发射冷却
  respawnIdle: 0,
  flagBonus: 0, // 顶旗杆高度分段奖励
  dead: false,
  won: false,
  flagCol: 0,
  key: {} as Record<string, boolean>,
  finalTime: 0,
};

let rafId = 0;
let lastTime = 0;

interface Enemy {
  x: number;
  y: number;
  w: number;
  h: number;
  vx: number;
  vy: number;
  alive: boolean;
  kind: "goomba" | "koopa"; // 栗子怪 / 乌龟
  state: "walk" | "shell" | "slide"; // 乌龟状态：行走 / 静止壳 / 滑行壳
}

/** 悬浮道具：变大蘑菇 / 火花花 / 1UP 绿蘑菇 */
interface Item {
  x: number;
  y: number;
  w: number;
  h: number;
  vx: number;
  vy: number;
  t: number;
  remove: boolean;
  type: "mushroom" | "flower" | "oneup";
}

/** 火球 */
interface Fireball {
  x: number;
  y: number;
  vx: number;
  t: number;
  remove: boolean;
}

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  t: number;
  max: number;
}

const groundTop = () => ROWS * TILE - TILE; // 最底行顶边（304）

/** 读取瓦片（越界视为实心底/空） */
function tileAt(c: number, r: number): TileChar {
  if (r < 0) return " ";
  if (c < 0 || c >= game.cols) return " ";
  const row = game.grid[r];
  return row ? row[c] : " ";
}

function isSolid(t: TileChar): boolean {
  return t === "X" || t === "B" || t === "Q" || t === "#" || t === "P" || t === "K" || t === "H";
}

// ============ 关卡生成（尽量复刻“超级马里奥兄弟 1-1”节奏） ============
function buildLevel() {
  const cols = game.cols;
  // 初始化空网格
  const grid: TileChar[][] = [];
  for (let r = 0; r < ROWS; r++) {
    grid.push(new Array<TileChar>(cols).fill(" "));
  }

  // 地面（最底行），保留 3 处宽窄不同的坑洞，模仿 1-1 的地形起伏
  for (let c = 0; c < cols; c++) {
    const pit =
      (c >= 30 && c <= 31) || // 2 格窄坑
      (c >= 62 && c <= 64) || // 3 格坑
      (c >= 96 && c <= 99); // 4 格宽坑
    grid[ROWS - 1][c] = pit ? " " : "X";
  }

  // 辅助：放一整行瓦片
  const putRow = (row: number, from: number, to: number, t: TileChar) => {
    for (let c = from; c <= to; c++) {
      if (c >= 0 && c < cols) grid[row][c] = t;
    }
  };

  // ---- 可顶层（第 14 行，地面上方 5 格）：问号块与砖块交错，仿 1-1 前段节奏 ----
  const layer: Array<[number, number, TileChar]> = [
    [5, 3, "B"], // 起步砖
    [7, 1, "Q"],
    [12, 2, "Q"],
    [16, 2, "B"],
    [18, 1, "Q"],
    [23, 3, "B"],
    [26, 1, "Q"],
    [27, 1, "Q"],
    [33, 3, "B"],
    [35, 1, "Q"],
    [37, 1, "Q"],
    [43, 3, "B"],
    [46, 1, "Q"],
    [48, 1, "Q"],
    [52, 3, "B"],
    [55, 1, "Q"],
    [58, 1, "Q"],
    [67, 3, "B"],
    [69, 1, "Q"],
    [71, 1, "Q"],
    [73, 2, "B"],
    [75, 1, "Q"],
    [80, 3, "B"],
    [83, 1, "Q"],
    [86, 1, "Q"],
    [88, 2, "B"],
    [91, 1, "Q"],
    [93, 1, "Q"],
    [104, 3, "B"],
    [106, 1, "Q"],
    [109, 1, "Q"],
    [113, 3, "B"],
    [115, 1, "Q"],
    [118, 2, "B"],
    [120, 1, "Q"],
    [123, 1, "Q"],
    [128, 3, "B"],
    [130, 2, "B"],
    [134, 1, "Q"],
    [136, 1, "Q"],
    [141, 3, "B"],
    [144, 1, "Q"],
    [147, 1, "Q"],
    [150, 3, "B"],
  ];
  for (const [from, count, t] of layer) putRow(14, from, from + count - 1, t);

  // ---- 高台（第 7 行）：空中砖道，上方再悬银币 ----
  const upper: Array<[number, number]> = [
    [22, 4],
    [60, 4],
    [77, 4],
    [117, 4],
    [143, 4],
  ];
  for (const [from, count] of upper) putRow(7, from, from + count - 1, "B");
  // 高台上空一字悬挂金币
  for (const c of [24, 25, 26, 62, 63, 79, 80, 119, 120, 145, 146]) grid[6][c] = "o";

  // ---- 可顶层上方短线金币（跳起可吃到） ----
  for (const c of [11, 12, 17, 18, 38, 39, 49, 50, 68, 69, 84, 85, 105, 106, 119, 120, 135, 136]) {
    grid[13][c] = "o";
  }

  // ---- 水管（6 座，高 2~4 格，分布在砖块之间的地面空隙） ----
  const pipes: Array<[number, number]> = [
    [14, 2],
    [20, 3],
    [40, 4],
    [66, 2],
    [110, 4],
    [149, 3],
  ];
  for (const [c, h] of pipes) {
    for (let r = ROWS - 1 - h; r < ROWS - 1; r++) grid[r][c] = "P";
  }

  // ---- 终点旗杆 + 城堡前台阶 + 城堡 ----
  game.flagCol = 165;
  // 旗杆（高 10 格）
  for (let r = ROWS - 11; r <= ROWS - 2; r++) grid[r][165] = "P";
  // 城堡前台阶（逐级升高 3 级）
  grid[14][161] = "X";
  grid[13][162] = "X";
  grid[12][163] = "X";
  // 城堡城墙（3 列宽，含顶部雉堞）
  for (let r = ROWS - 6; r < ROWS; r++) grid[r][168] = "K";
  grid[ROWS - 8][169] = "K";
  grid[ROWS - 8][170] = "K";
  for (let r = ROWS - 6; r < ROWS; r++) grid[r][171] = "K";

  game.grid = grid;

  // 指定某些问号块产出“变大蘑菇”或“火花花”（列号取自上方的 Q 块）
  game.mushroomBlocks = new Set([7, 26, 55, 83, 106, 134]);
  game.flowerBlocks = new Set([27, 48, 75, 93, 115, 136, 144]);
  // 某些砖块内藏金币：顶它们出金币后变空（而非碎裂）
  game.coinBricks = new Set([23, 43, 67, 88, 118, 141]);

  // ---- 隐藏区域：不可见砖块（顶到才出金币并显现，可借其登高） ----
  const hiddenCols = [11, 29, 38, 51, 59, 78, 90, 97, 107, 122, 132, 140, 153];
  for (const c of hiddenCols) grid[14][c] = "H";

  // 隐藏区域藏宝：若干隐蔽位置放金币串，跳到隐藏砖上可吃到
  for (const c of [10, 28, 39, 52, 60, 79, 89, 98, 106, 121, 131, 141]) grid[13][c] = "o";

  // ---- 隐藏连跳通道 + 1UP：在第 125 列搭一条隐藏砖阶梯，跳到顶端的问号块顶出 1UP 绿蘑菇 ----
  game.oneupBlocks = new Set([125]);
  const ladderCol = 125;
  for (const r of [14, 11, 8, 5]) grid[r][ladderCol] = "H"; // 竖直阶梯（差 3 格，逐级可跳）
  grid[4][ladderCol] = "Q"; // 顶端问号块 → 1UP
  // 阶梯旁点缀金币（跳上去可顺路吃到）
  for (const [c, r] of [[124, 13], [126, 12], [125, 7], [126, 6]] as Array<[number, number]>) {
    grid[r][c] = "o";
  }

  // ---- 敌人：栗子怪与绿乌龟（乌龟可踩成龟壳滑行） ----
  const spawnDefs: Array<[number, "goomba" | "koopa"]> = [
    [9, "goomba"], [10, "goomba"], [25, "koopa"], [39, "goomba"], [45, "goomba"],
    [46, "goomba"], [57, "koopa"], [58, "goomba"], [72, "goomba"], [73, "goomba"],
    [87, "koopa"], [102, "goomba"], [103, "goomba"], [115, "koopa"], [116, "goomba"],
    [132, "koopa"], [133, "goomba"], [145, "goomba"], [146, "goomba"], [158, "koopa"],
  ];
  game.enemies = spawnDefs.map(([c, kind]) => ({
    x: c * TILE,
    y: groundTop() - 14,
    w: 14,
    h: 14,
    vx: kind === "koopa" ? -(0.3 + Math.random() * 0.3) : -(0.5 + Math.random() * 0.5),
    vy: 0,
    alive: true,
    kind,
    state: "walk" as "walk" | "shell" | "slide",
  }));

  game.items = [];
  game.fireballs = [];
  game.particles = [];
}

// ============ 游戏流程 ============
function newGame() {
  buildLevel();
  game.state = "play";
  game.score = 0;
  game.coins = 0;
  game.lives = 3;
  game.time = 200;
  game.level = 1;
  game.dead = false;
  game.won = false;
  game.fireballs = [];
  game.fireCd = 0;
  spawnMario();
}

/** 设置马里奥体型（0=小 / 1=大），保持脚底对齐 */
function setMarioSize(size: 0 | 1) {
  const m = game.mario;
  const feet = m.y + m.h;
  if (size === 1) {
    m.w = 16;
    m.h = 32;
  } else {
    m.w = 14;
    m.h = 20;
    // 缩回小号时保留火球能力（原版规则：缩小时保留火花花进度）
  }
  m.size = size;
  m.y = feet - m.h;
}

function spawnMario() {
  const m = game.mario;
  m.size = 0;
  m.w = 14;
  m.h = 20;
  m.inv = 0;
  m.fire = 0;
  m.x = 2 * TILE;
  m.y = groundTop() - m.h;
  m.vx = 0;
  m.vy = 0;
  m.onGround = true;
  game.camera = 0;
}

/** 根据马里奥抓到旗杆时的高度计算分段奖励（越高越多） */
function flagSegment(marioY: number): number {
  const top = (ROWS - 11) * TILE; // 旗杆顶端 y
  const bottom = groundTop(); // 旗杆底端（地面）y
  const ratio = (marioY - top) / (bottom - top);
  if (ratio < 0.2) return 5000; // 几乎抓到顶
  if (ratio < 0.4) return 4000;
  if (ratio < 0.6) return 2000;
  if (ratio < 0.8) return 800;
  return 100; // 旗杆底部附近
}

/** 通关：结算并提交成绩 */
function doWin(heightBonus: number) {
  game.state = "clear";
  game.won = true;
  game.finalTime = game.time;
  game.flagBonus = heightBonus;
  const timeBonus = game.time * 10;
  game.score += heightBonus + timeBonus;
  void marioApi
    .submitScore({
      score: game.score,
      level: game.level,
      coins: game.coins,
      time_ms: 0,
    })
    .catch(() => undefined);
}

function playerDie() {
  game.lives -= 1;
  game.dead = true;
  game.state = "dead";
  game.respawnIdle = 60;
}

function respawnOrOver() {
  if (game.lives > 0) {
    game.dead = false;
    game.state = "play";
    spawnMario();
  } else {
    game.state = "clear";
    game.won = false;
  }
}

// ============ 输入 ============
function onKeyDown(e: KeyboardEvent) {
  const k = e.key;
  if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", " "].includes(k)) e.preventDefault();
  game.key[k] = true;

  if (game.state === "title" && (k === "Enter" || k === " ")) {
    newGame();
  } else if (game.state === "clear" && k === "Enter") {
    newGame();
  }
  // 跳跃优先响应（含长按跳更高）
  if (game.state === "play" && (k === " " || k === "ArrowUp" || k === "w" || k === "W")) {
    tryJump();
  }
  // 火球射击（X / C，需拥有火球能力）
  if (game.state === "play" && (k === "x" || k === "X" || k === "c" || k === "C")) {
    fire();
  }
}

function onKeyUp(e: KeyboardEvent) {
  game.key[e.key] = false;
  // 松开跳跃键→降低上升速度（可变跳跃）
  if (e.key === " " || e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
    if (game.mario.vy < 0) game.mario.vy *= 0.5;
  }
}

function tryJump() {
  const m = game.mario;
  if (m.onGround) {
    // 大马里奥跳得更高（贴原版）：小号 -8 / 大号 -9.6
    m.vy = m.size === 1 ? -9.6 : -8;
    m.onGround = false;
    game.timer = 12; // 跳跃缓冲
  }
}

/** 发射火球（需拥有火球能力，受冷却限制） */
function fire() {
  const m = game.mario;
  if (game.mario.fire !== 1 || game.fireCd > 0) return;
  game.fireCd = 12;
  game.fireballs.push({
    x: m.x + (m.facing > 0 ? m.w : -6),
    y: m.y + m.h / 2 - 3,
    vx: m.facing > 0 ? 3.2 : -3.2,
    t: 0,
    remove: false,
  });
}

function isLeft() {
  return !!(game.key["ArrowLeft"] || game.key["a"] || game.key["A"]);
}
function isRight() {
  return !!(game.key["ArrowRight"] || game.key["d"] || game.key["D"]);
}

// ============ 物理 ============
function solidAt(c: number, r: number): boolean {
  return isSolid(tileAt(Math.floor(c / TILE), Math.floor(r / TILE)));
}

function moveAndCollide(ent: { x: number; y: number; w: number; h: number; vx: number; vy: number; onGround: boolean }) {
  // X 轴
  ent.x += ent.vx;
  if (ent.vx > 0) {
    if (solidAt(ent.x + ent.w, ent.y) || solidAt(ent.x + ent.w, ent.y + ent.h - 1)) {
      ent.x = Math.floor((ent.x + ent.w) / TILE) * TILE - ent.w - 0.01;
      ent.vx = 0;
    }
  } else if (ent.vx < 0) {
    if (solidAt(ent.x, ent.y) || solidAt(ent.x, ent.y + ent.h - 1)) {
      ent.x = (Math.floor(ent.x / TILE) + 1) * TILE + 0.01;
      ent.vx = 0;
    }
  }

  // Y 轴
  ent.vy += GRAV;
  if (ent.vy > MAX_FALL) ent.vy = MAX_FALL;
  ent.y += ent.vy;
  ent.onGround = false;
  if (ent.vy >= 0) {
    // 下落：压到实心顶
    if (solidAt(ent.x + 1, ent.y + ent.h) || solidAt(ent.x + ent.w - 1, ent.y + ent.h)) {
      ent.y = Math.floor((ent.y + ent.h) / TILE) * TILE - ent.h - 0.01;
      ent.vy = 0;
      ent.onGround = true;
    }
  } else {
    // 上跳：顶到块下沿
    if (solidAt(ent.x + 1, ent.y) || solidAt(ent.x + ent.w - 1, ent.y)) {
      const col = Math.floor((ent.x + ent.w / 2) / TILE);
      const row = Math.floor(ent.y / TILE);
      hitBlockFromBelow(col, row);
      ent.y = Math.floor(ent.y / TILE) * TILE + TILE + 0.01;
      ent.vy = 0;
    }
  }
}

function hitBlockFromBelow(col: number, row: number) {
  const t = tileAt(col, row);
  if (t === "H") {
    // 隐藏砖：顶出金币并变为“已使用”砖（从此可见）
    game.grid[row][col] = "#";
    game.coins += 1;
    game.score += 150;
    spawnCoinBurst(col * TILE + 8, row * TILE);
  } else if (t === "Q") {
    // 问号块：变已使用。产出优先级：1UP > 火花花 > 大蘑菇 > 金币
    game.grid[row][col] = "#";
    if (game.oneupBlocks.has(col)) {
      game.score += 100;
      game.items.push(itemFromBlock(col, row, "oneup"));
    } else if (game.flowerBlocks.has(col)) {
      game.score += 100;
      game.items.push(itemFromBlock(col, row, "flower"));
    } else if (game.mushroomBlocks.has(col)) {
      game.score += 100;
      game.items.push(itemFromBlock(col, row, "mushroom"));
    } else {
      game.coins += 1;
      game.score += 150;
      spawnCoinBurst(col * TILE + 8, row * TILE);
    }
  } else if (t === "B") {
    if (game.coinBricks.has(col)) {
      // 金币砖：顶出金币后变空（不再碎裂）
      game.grid[row][col] = "#";
      game.coins += 1;
      game.score += 150;
      spawnCoinBurst(col * TILE + 8, row * TILE);
    } else {
      // 普通砖块：顶碎
      game.grid[row][col] = " ";
      game.score += 60;
      spawnBrickBits(col * TILE, row * TILE);
    }
  }
}

/** 从问号块生成道具（1UP / 蘑菇滑动 / 火花花原地） */
function itemFromBlock(col: number, row: number, type: "mushroom" | "flower" | "oneup"): Item {
  return {
    x: col * TILE + 1,
    y: row * TILE - 14,
    w: 14,
    h: 14,
    vx: type === "mushroom" ? 0.8 : 0,
    vy: 0,
    t: 0,
    remove: false,
    type,
  };
}

// ============ 粒子 ============
function spawnCoinBurst(x: number, y: number) {
  game.particles.push({ x, y, vx: 0, vy: -5.5, t: 0, max: 20 });
}
function spawnBrickBits(x: number, y: number) {
  for (let i = 0; i < 4; i++) {
    game.particles.push({ x, y, vx: (i % 2 === 0 ? -1 : 1) * (1 + i * 0.4), vy: -3 - i * 0.5, t: 0, max: 40 });
  }
}

// ============ 主循环 ============
function update() {
  const m = game.mario;
  game.frame++;

  if (game.state === "play") {
    // 倒计时
    game.timer -= 1;
    if (game.timer <= -30) {
      game.time -= 1;
      game.timer = 0;
      if (game.time <= 0) {
        playerDie();
        return;
      }
    }

    // 水平输入
    const target = (isRight() ? 2.2 : 0) + (isLeft() ? -2.2 : 0);
    m.vx = target;
    m.facing = m.vx === 0 ? m.facing : m.vx < 0 ? -1 : 1;
    if (m.onGround) m.anim += Math.abs(m.vx);

    moveAndCollide(m);

    // 碰金币（网格）
    const c0 = Math.max(0, Math.floor(m.x / TILE));
    const c1 = Math.min(game.cols - 1, Math.floor((m.x + m.w) / TILE));
    const r0 = Math.floor(m.y / TILE);
    const r1 = Math.min(ROWS - 1, Math.floor((m.y + m.h) / TILE));
    for (let r = r0; r <= r1; r++) {
      for (let c = c0; c <= c1; c++) {
        if (tileAt(c, r) === "o") {
          game.grid[r][c] = " ";
          game.coins += 1;
          game.score += 100;
          spawnCoinBurst(c * TILE + 8, r * TILE);
        }
      }
    }

    // 无敌帧递减
    if (m.inv > 0) m.inv--;

    // 道具（蘑菇/火花花）物理与收集
    for (const it of game.items) {
      it.t++;
      // 冒出动画：前 24 帧向上弹起
      if (it.t < 24) {
        it.y -= 2;
        continue;
      }
      // 火花花：原地待前（不滑动，轻微浮动）；蘑菇：重力 + 沿地面滑动
      if (it.type === "flower") {
        it.y += Math.sin(it.t * 0.15) * 0.4;
      } else {
        it.x += it.vx;
        it.y += it.vy;
        it.vy += GRAV;
        if (it.vy > MAX_FALL) it.vy = MAX_FALL;
        const stepR = Math.floor((it.y + it.h) / TILE);
        const under = isSolid(tileAt(Math.floor((it.x + 2) / TILE), stepR)) ||
          isSolid(tileAt(Math.floor((it.x + it.w - 2) / TILE), stepR));
        if (under) {
          it.y = stepR * TILE - it.h - 0.01;
          it.vy = 0;
        }
        // 撞墙反向
        const frontC = it.vx >= 0 ? Math.floor((it.x + it.w) / TILE) : Math.floor(it.x / TILE);
        if (isSolid(tileAt(frontC, Math.floor(it.y / TILE)))) it.vx *= -1;
      }

      // 收集判定（蘑菇 & 火花花共用）
      if (
        m.x < it.x + it.w && m.x + m.w > it.x &&
        m.y < it.y + it.h && m.y + m.h > it.y
      ) {
        it.remove = true;
        game.score += 1000;
        spawnCoinBurst(it.x + 7, it.y);
        if (it.type === "oneup") {
          // 1UP 绿蘑菇：增加一条生命（封顶 99）
          game.lives = Math.min(99, game.lives + 1);
        } else if (it.type === "flower") {
          // 火花花：小→变大；大→获得火球射击能力
          if (game.mario.size === 0) setMarioSize(1);
          else game.mario.fire = 1;
        } else {
          // 蘑菇：小→变大；大→仅加分
          if (game.mario.size === 0) setMarioSize(1);
        }
      }
    }
    game.items = game.items.filter((i) => !i.remove);

    // 火球更新（只在拥有火球能力时刷新）
    if (game.fireCd > 0) game.fireCd--;
    for (const f of game.fireballs) {
      f.t++;
      f.x += f.vx;
      // 撞墙消失
      const fy = Math.floor((f.y + 4) / TILE);
      const fCol = f.vx > 0 ? Math.floor((f.x + 6) / TILE) : Math.floor(f.x / TILE);
      if (isSolid(tileAt(fCol, Math.floor(f.y / TILE))) || isSolid(tileAt(fCol, fy))) {
        f.remove = true;
      }
      // 撞敌击杀
      for (const e of game.enemies) {
        if (!e.alive) continue;
        if (f.x < e.x + e.w && f.x + 6 > e.x && f.y < e.y + e.h && f.y + 6 > e.y) {
          e.alive = false;
          game.score += 200;
          f.remove = true;
          break;
        }
      }
      if (f.t > 90) f.remove = true;
    }
    game.fireballs = game.fireballs.filter((f) => !f.remove);

    // 敌人：前方有墙才反向（修复“原地抖/不动”），并受重力随地形起伏落坑
    for (const e of game.enemies) {
      if (!e.alive) continue;
      // 静止龟壳不自主移动
      if (e.state !== "shell") {
        e.x += e.vx;
        // 前方墙体检测（用身体上/中两行，避免把脚下地面误判为墙）
        const frontC = e.vx >= 0 ? Math.floor((e.x + e.w) / TILE) : Math.floor(e.x / TILE);
        const wallR1 = Math.floor(e.y / TILE);
        const wallR2 = Math.floor((e.y + e.h - 2) / TILE);
        if (isSolid(tileAt(frontC, wallR1)) || isSolid(tileAt(frontC, wallR2))) {
          e.vx *= -1;
        }
      }
      // 重力：脚下有实心（地/平台/台阶）则停留，否则下落（会掉进坑）
      e.y += e.vy;
      e.vy += GRAV;
      if (e.vy > MAX_FALL) e.vy = MAX_FALL;
      const stepR = Math.floor((e.y + e.h) / TILE);
      const underL = isSolid(tileAt(Math.floor((e.x + 1) / TILE), stepR));
      const underR = isSolid(tileAt(Math.floor((e.x + e.w - 1) / TILE), stepR));
      if (underL || underR) {
        e.y = stepR * TILE - e.h - 0.01;
        e.vy = 0;
      }

      // 与马里奥碰撞
      if (
        m.x < e.x + e.w && m.x + m.w > e.x &&
        m.y < e.y + e.h && m.y + m.h > e.y
      ) {
        const falling = m.vy > 0;
        const above = m.y + m.h - e.y < e.h * 0.6;
        if (falling && above) {
          // 踩顶
          m.vy = -6;
          m.onGround = false;
          if (e.kind === "goomba") {
            e.alive = false;
            game.score += 200;
          } else if (e.state === "walk") {
            // 乌龟：缩壳
            e.state = "shell";
            e.vx = 0;
            game.score += 100;
          } else if (e.state === "shell") {
            // 静止壳：踢成滑行壳（朝被踩时面对的反方向）
            e.state = "slide";
            e.vx = m.x < e.x ? 4 : -4;
            game.score += 100;
          } else {
            // 滑行壳：踩停
            e.state = "shell";
            e.vx = 0;
            game.score += 100;
          }
        } else if (m.inv > 0) {
          // 无敌帧：免受伤害
        } else if (e.kind === "koopa" && e.state === "shell") {
          // 静止壳侧碰：不受伤，但被踢走滑行
          e.state = "slide";
          e.vx = m.x < e.x ? 4 : -4;
        } else if (game.mario.size === 1) {
          // 大马里奥被撞（goomba / 行走龟 / 滑行壳）：缩回小号 + 短暂无敌
          setMarioSize(0);
          m.inv = 120;
          m.vy = -4;
          m.onGround = false;
        } else {
          playerDie();
          return;
        }
      }
    }

    // 滑行龟壳撞击其他敌人（连锁击杀）
    for (const e of game.enemies) {
      if (!e.alive || e.kind !== "koopa" || e.state !== "slide") continue;
      for (const o of game.enemies) {
        if (o === e || !o.alive) continue;
        if (o.x < e.x + e.w && o.x + o.w > e.x && o.y < e.y + e.h && o.y + o.h > e.y) {
          o.alive = false;
          game.score += 200;
        }
      }
    }

    // 移除已被踩灭/击杀的敌人
    game.enemies = game.enemies.filter((e) => e.alive);

    // 掉落判定（掉出屏幕或掉进坑）
    if (m.y > VIEW_H) {
      playerDie();
      return;
    }

    // 终点判定：触到旗杆柱体，按抓到的高度分段给分
    {
      const pX1 = game.flagCol * TILE;
      const pX2 = pX1 + TILE;
      const pTopY = (ROWS - 11) * TILE;
      const pBotY = groundTop();
      if (m.x + m.w > pX1 && m.x < pX2 && m.y + m.h > pTopY && m.y < pBotY) {
        doWin(flagSegment(m.y));
        return;
      }
    }

    // 摄像机跟随
    const targetCam = Math.max(0, Math.min(m.x - VIEW_W * 0.4, game.cols * TILE - VIEW_W));
    game.camera = targetCam;
  } else if (game.state === "dead") {
    game.respawnIdle--;
    m.y += m.vy;
    m.vy += GRAV * 1.2;
    if (m.y > VIEW_H + 40) {
      respawnOrOver();
      return;
    }
  } else if (game.state === "clear") {
    // 结算动画：马里奥原地小跳示意，无需逻辑
  }

  // 粒子更新
  for (const p of game.particles) {
    p.t++;
    p.x += p.vx;
    p.y += p.vy;
    p.vy += 0.25;
  }
  game.particles = game.particles.filter((p) => p.t < p.max);
}

// ============ 渲染 ============
function render() {
  if (!ctx) return;
  ctx.imageSmoothingEnabled = false;

  // 天
  const sky = ctx.createLinearGradient(0, 0, 0, VIEW_H);
  sky.addColorStop(0, "#6fb8ff");
  sky.addColorStop(1, "#bde3ff");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, VIEW_W, VIEW_H);

  // 云（简单静态）
  ctx.fillStyle = "rgba(255,255,255,0.9)";
  drawCloud(40, 55, 26);
  drawCloud(200, 90, 22);
  drawCloud(360, 45, 18);
  drawCloud(150, 130, 30);
  drawCloud(280, 150, 24);

  // 摄像机偏移
  const cam = Math.floor(game.camera);

  // 绘制瓦片
  for (let r = 0; r < ROWS; r++) {
    for (let c = Math.floor(cam / TILE) - 1; c <= Math.floor((cam + VIEW_W) / TILE) + 1; c++) {
      if (c < 0 || c >= game.cols) continue;
      const t = game.grid[r][c];
      if (t === " ") continue;
      const px = c * TILE - cam;
      const py = r * TILE;
      drawTile(t, px, py);
    }
  }

  // 敌人
  for (const e of game.enemies) {
    drawEnemy(e);
  }

  // 道具（蘑菇/火花花）
  for (const it of game.items) {
    drawItem(it);
  }

  // 火球
  for (const f of game.fireballs) {
    const fx = Math.round(f.x - cam);
    const fy = Math.round(f.y);
    ctx.fillStyle = "#ff7a1a";
    ctx.beginPath();
    ctx.arc(fx + 3, fy + 3, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#ffdf80";
    ctx.beginPath();
    ctx.arc(fx + 2, fy + 2, 2.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // 马里奥
  drawMario();

  // 粒子
  for (const p of game.particles) {
    ctx.fillStyle = "#ffcc00";
    ctx.fillRect(p.x + 2 - cam, p.y, 6, 6);
  }

  // HUD
  drawHud(cam);
}

function drawCloud(x: number, y: number, r: number) {
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.arc(x + r * 0.8, y - r * 0.3, r * 0.7, 0, Math.PI * 2);
  ctx.arc(x + r * 1.5, y, r * 0.8, 0, Math.PI * 2);
  ctx.fill();
}

function drawBrick(px: number, py: number, brown: string, mortar: string) {
  if (!ctx) return;
  ctx.fillStyle = brown;
  ctx.fillRect(px + 1, py + 1, TILE - 2, TILE - 2);
  ctx.fillStyle = mortar;
  // 砖缝
  const half = TILE / 2;
  ctx.fillRect(px + half, py + 1, 1, TILE - 2);
  ctx.fillRect(px + 1, py + TILE / 2 - 1, half - 1, 1);
  ctx.fillRect(px + half + 1, py + 1, half - 1, 1);
}

function drawTile(t: TileChar, px: number, py: number) {
  if (!ctx) return;
  // 隐藏砖：不可见（顶到变 '#' 后才会绘制）
  if (t === "H") return;
  switch (t) {
    case "X":
    case "P":
      // 地面砖 / 水管
      ctx.fillStyle = "#c94b1c";
      ctx.fillRect(px, py, TILE, TILE);
      ctx.fillStyle = "#ffcf7a";
      ctx.fillRect(px, py, TILE, 2);
      ctx.fillRect(px, py + 4, TILE, 2);
      ctx.fillStyle = "#8a2f0f";
      ctx.fillRect(px + 2, py + 12, TILE - 4, 2);
      break;
    case "B":
      drawBrick(px, py, "#c94b1c", "#7a2600");
      break;
    case "Q":
      // 问号块（金色）
      ctx.fillStyle = "#ffb400";
      ctx.fillRect(px, py, TILE, TILE);
      ctx.fillStyle = "#ffe899";
      ctx.fillRect(px + 2, py + 2, TILE - 4, TILE - 4);
      ctx.fillStyle = "#8a5a00";
      ctx.font = "10px monospace";
      ctx.textAlign = "center";
      ctx.fillText("?", px + TILE / 2, py + 12);
      break;
    case "#":
      ctx.fillStyle = "#a56a1f";
      ctx.fillRect(px, py, TILE, TILE);
      ctx.fillStyle = "#7a4a12";
      ctx.fillRect(px + 2, py + 2, TILE - 4, TILE - 4);
      break;
    case "o":
      ctx.fillStyle = "#ffcc00";
      ctx.beginPath();
      ctx.arc(px + TILE / 2, py + TILE / 2, TILE / 2 - 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#fff6cc";
      ctx.beginPath();
      ctx.arc(px + TILE / 2 - 1, py + TILE / 2 - 2, 2, 0, Math.PI * 2);
      ctx.fill();
      break;
    case "K":
      // 城堡端墙
      ctx.fillStyle = "#e0e0e0";
      ctx.fillRect(px, py, TILE, TILE);
      ctx.fillStyle = "#b0b0b0";
      ctx.fillRect(px, py, TILE, 3);
      ctx.fillRect(px, py + TILE - 3, TILE, 3);
      break;
  }
}

function drawEnemy(e: Enemy) {
  if (!ctx) return;
  const px = e.x - game.camera;
  if (e.kind === "koopa") {
    // ===== 乌龟 =====
    if (e.state === "shell" || e.state === "slide") {
      // 龟壳（静止或滑行）
      ctx.fillStyle = "#3f9e33";
      ctx.beginPath();
      ctx.ellipse(px + e.w / 2, e.y + e.h, e.w / 2, e.h / 2 - 1, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#e8f2c0";
      ctx.beginPath();
      ctx.ellipse(px + e.w / 2, e.y + 6, e.w / 2 - 2, e.h / 3, 0, 0, Math.PI * 2);
      ctx.fill();
      // 滑行时画速度线
      if (e.state === "slide") {
        ctx.strokeStyle = "rgba(255,255,255,0.6)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 1; i <= 3; i++) {
          const lx = px + (e.vx > 0 ? -i * 4 : e.w + i * 4);
          ctx.moveTo(lx - 2, e.y + e.h - 2);
          ctx.lineTo(lx + 2, e.y + e.h - 5);
        }
        ctx.stroke();
      }
      return;
    }
    // 行走乌龟
    // 绿壳
    ctx.fillStyle = "#3f9e33";
    ctx.beginPath();
    ctx.arc(px + e.w / 2, e.y + 3, e.w / 2 - 1, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#e8f2c0";
    ctx.beginPath();
    ctx.ellipse(px + e.w / 2, e.y + 1, e.w / 2 - 3, 3, 0, 0, Math.PI * 2);
    ctx.fill();
    // 头
    ctx.fillStyle = "#c8e0b0";
    ctx.fillRect(px + e.w / 2 + (e.vx > 0 ? 1 : -5), e.y + 3, 5, 4);
    // 眼睛
    ctx.fillStyle = "#333";
    ctx.fillRect(px + e.w / 2 + (e.vx > 0 ? 3 : -1), e.y + 4, 2, 2);
    // 脚
    ctx.fillStyle = "#5a7a2a";
    const step = Math.sin(game.frame * 0.4) > 0 ? 0 : 2;
    ctx.fillRect(px + 2 + step, e.y + e.h - 3, 4, 3);
    ctx.fillRect(px + e.w - 5 - step, e.y + e.h - 3, 4, 3);
    return;
  }

  // ===== 栗子怪 goomba =====
  const bob = Math.sin(game.frame * 0.2);
  ctx.fillStyle = "#a03e1f";
  ctx.beginPath();
  ctx.arc(px + e.w / 2, e.y - 4, 8, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#6b2412";
  ctx.beginPath();
  ctx.ellipse(px + e.w / 2, e.y + e.h, e.w / 2, e.h / 2, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  ctx.arc(px + e.w / 2 - 3, e.y + 2, 1.8, 0, Math.PI * 2);
  ctx.arc(px + e.w / 2 + 3, e.y + 2, 1.8, 0, Math.PI * 2);
  ctx.fill();
  void bob;
}

function drawItem(it: Item) {
  if (!ctx) return;
  const px = Math.round(it.x - game.camera);
  const py = Math.round(it.y);
  if (it.type === "flower") {
    // ===== 火花花：白花 + 黄芯，花茎 =====
    ctx.fillStyle = "#3f9e33";
    ctx.fillRect(px + it.w / 2 - 1, py + it.h - 4, 2, 4);
    ctx.fillStyle = "#fff";
    ctx.beginPath();
    ctx.arc(px + it.w / 2 - 3, py + it.h / 2 - 4, 3, 0, Math.PI * 2);
    ctx.arc(px + it.w / 2 + 3, py + it.h / 2 - 4, 3, 0, Math.PI * 2);
    ctx.arc(px + it.w / 2, py + it.h / 2 - 7, 3, 0, Math.PI * 2);
    ctx.arc(px + it.w / 2, py + it.h / 2 - 1, 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#ffcf3a";
    ctx.beginPath();
    ctx.arc(px + it.w / 2, py + it.h / 2 - 4, 3, 0, Math.PI * 2);
    ctx.fill();
    return;
  }
  if (it.type === "oneup") {
    // ===== 1UP 绿蘑菇：绿帽 + 白点 + 白柄（加命） =====
    ctx.fillStyle = "#3f9e33";
    ctx.beginPath();
    ctx.arc(px + it.w / 2, py + 4, it.w / 2, Math.PI, 0);
    ctx.fill();
    ctx.fillRect(px, py + 4, it.w, it.h / 2 - 1);
    ctx.fillStyle = "#fff";
    ctx.beginPath();
    ctx.arc(px + it.w / 2, py + 3, 1.6, 0, Math.PI * 2);
    ctx.arc(px + 3, py + 5, 1.4, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#fff6e0";
    ctx.fillRect(px + 2, py + it.h / 2 - 1, it.w - 4, it.h - it.h / 2 + 1);
    // "1UP" 字样
    ctx.fillStyle = "#fff";
    ctx.font = "bold 6px monospace";
    ctx.textAlign = "center";
    ctx.fillText("1UP", px + it.w / 2, py + it.h - 2);
    return;
  }
  // ===== 蘑菇：红帽 + 白点 + 白柄 =====
  ctx.fillStyle = "#e23b2e";
  ctx.beginPath();
  ctx.arc(px + it.w / 2, py + 4, it.w / 2, Math.PI, 0);
  ctx.fill();
  ctx.fillRect(px, py + 4, it.w, it.h / 2 - 1);
  // 帽上白点
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  ctx.arc(px + it.w / 2, py + 3, 1.6, 0, Math.PI * 2);
  ctx.arc(px + 3, py + 5, 1.4, 0, Math.PI * 2);
  ctx.fill();
  // 白色蘑菇柄
  ctx.fillStyle = "#fff6e0";
  ctx.fillRect(px + 2, py + it.h / 2 - 1, it.w - 4, it.h - it.h / 2 + 1);
  // 眼睛
  ctx.fillStyle = "#333";
  ctx.fillRect(px + 3, py + 4, 2, 2);
  ctx.fillRect(px + it.w - 5, py + 4, 2, 2);
}

function drawMario() {
  if (!ctx) return;
  const m = game.mario;
  // 无敌帧：闪烁（每隔几帧消失一次）
  if (m.inv > 0 && Math.floor(m.inv / 4) % 2 === 0) return;
  const px = Math.round(m.x - game.camera);
  const py = Math.round(m.y);
  const big = m.size === 1;

  if (big) {
    // ===== 大马里奥（更高，红帽+蓝工装） =====
    // 帽子
    ctx.fillStyle = "#e23b2e";
    ctx.fillRect(px + 1, py, m.w - 2, 5);
    ctx.fillRect(px - 1, py + 4, m.w + 2, 2);
    // 帽檐"M"
    ctx.fillStyle = "#fff";
    ctx.fillRect(px + (m.facing > 0 ? 4 : m.w - 8), py + 1, 5, 3);
    // 脸
    ctx.fillStyle = "#f7c59f";
    ctx.fillRect(px + 1, py + 6, m.w - 2, 8);
    // 眼睛
    ctx.fillStyle = "#333";
    ctx.fillRect(px + (m.facing > 0 ? m.w - 7 : 5), py + 8, 2, 3);
    // 胡子
    ctx.fillStyle = "#7a3b1a";
    ctx.fillRect(px + 1, py + 12, m.w - 2, 2);
    // 工装（蓝）
    ctx.fillStyle = "#2a6bd8";
    ctx.fillRect(px + 1, py + 15, m.w - 2, m.h - 17);
    // 工装扣子
    ctx.fillStyle = "#ffe899";
    ctx.fillRect(px + (m.facing > 0 ? m.w - 8 : 6), py + 18, 2, 2);
    // 脚
    ctx.fillStyle = "#8a4a24";
    if (m.onGround) {
      const step = Math.sin(game.mario.anim * 0.5) > 0 ? 1.5 : -1.5;
      ctx.fillRect(px - 1 + step, py + m.h - 4, 9, 4);
      ctx.fillRect(px + m.w - 1 - step, py + m.h - 4, 9, 4);
    } else {
      ctx.fillRect(px - 2, py + m.h - 4, 8, 4);
      ctx.fillRect(px + m.w - 6, py + m.h - 4, 8, 4);
    }
    return;
  }

  // ===== 小马里奥 =====
  // 帽子（红）
  ctx.fillStyle = "#e23b2e";
  ctx.fillRect(px + 1, py, m.w - 2, 4);
  // 帽檐
  ctx.fillRect(px - 1, py + 3, m.w + 2, 2);
  // 脸
  ctx.fillStyle = "#f7c59f";
  ctx.fillRect(px + 1, py + 5, m.w - 2, 5);
  // 眼睛
  ctx.fillStyle = "#333";
  ctx.fillRect(px + (m.facing > 0 ? m.w - 6 : 3), py + 6, 2, 2);
  // 胡子嘴巴
  ctx.fillStyle = "#7a3b1a";
  ctx.fillRect(px + 1, py + 10, m.w - 2, 2);
  // 身体（蓝工装）
  ctx.fillStyle = "#2a6bd8";
  ctx.fillRect(px + 1, py + 12, m.w - 2, m.h - 13);
  // 脚
  ctx.fillStyle = "#8a4a24";
  if (!m.onGround) {
    ctx.fillRect(px - 1, py + m.h - 3, m.w / 2, 3);
    ctx.fillRect(px + m.w / 2 - 1, py + m.h - 4, m.w / 2, 3);
  } else {
    const step = Math.sin(game.mario.anim * 0.5) > 0 ? 1 : -1;
    ctx.fillRect(px + step, py + m.h - 3, m.w / 2 + 1, 3);
    ctx.fillRect(px + m.w / 2 - step, py + m.h - 3, m.w / 2 + 1, 3);
  }
}

function drawHud(cam: number) {
  if (!ctx) return;
  void cam;
  ctx.fillStyle = "rgba(0,0,0,0.35)";
  ctx.fillRect(0, 0, VIEW_W, 26);
  ctx.fillStyle = "#fff";
  ctx.font = "bold 13px monospace";
  ctx.textAlign = "left";
  ctx.fillText(`SCORE ${String(game.score).padStart(6, "0")}`, 8, 18);
  ctx.textAlign = "center";
  ctx.fillText(`COINS x${game.coins}`, VIEW_W / 2, 18);
  ctx.textAlign = "right";
  ctx.fillText(`LIVES ${game.lives}   ⏱ ${game.time}`, VIEW_W - 8, 18);
}

function drawOverlayTitle() {
  if (!ctx) return;
  ctx.fillStyle = "rgba(0,0,0,0.55)";
  ctx.fillRect(0, 0, VIEW_W, VIEW_H);
  ctx.textAlign = "center";
  ctx.fillStyle = "#ffcc00";
  ctx.font = "bold 30px monospace";
  ctx.fillText("SUPER MARIO", VIEW_W / 2, 120);
  ctx.fillStyle = "#fff";
  ctx.font = "14px monospace";
  ctx.fillText("复刻版平台跳跃", VIEW_W / 2, 146);
  ctx.fillStyle = "#e23b2e";
  ctx.fillText("←→ 移动  |  Space/↑ 跳跃  |  X/C 火球", VIEW_W / 2, 178);
  ctx.fillText("顶 ?块出金币/蘑菇/火花花 · 顶 B砖可碎 · 踩敌人消灭", VIEW_W / 2, 198);
  ctx.fillText("踩乌龟可缩壳/滑行 · 触旗杆按高度分段得分", VIEW_W / 2, 218);
  ctx.fillText("隐藏砖可登高 · 顶端问号块藏着 1UP 绿蘑菇加命", VIEW_W / 2, 236);
  ctx.fillStyle = "#7aff7a";
  ctx.font = "bold 18px monospace";
  ctx.fillText("按 ENTER 开始", VIEW_W / 2, 262);
}

function drawOverlayEnd(gameOver: boolean) {
  if (!ctx) return;
  ctx.fillStyle = "rgba(0,0,0,0.6)";
  ctx.fillRect(0, 0, VIEW_W, VIEW_H);
  ctx.textAlign = "center";
  ctx.font = "bold 26px monospace";
  ctx.fillStyle = gameOver ? "#ff5a5a" : "#7aff7a";
  ctx.fillText(gameOver ? "GAME OVER" : "LEVEL CLEAR!", VIEW_W / 2, 110);

  ctx.font = "16px monospace";
  ctx.fillStyle = "#fff";
  const lines = gameOver
    ? [`得分 ${game.score}`, `金币 ${game.coins}`, `为你通关的下一关加油~`]
    : [
        `得分 ${game.score}`,
        `旗杆高度奖励 +=${game.flagBonus}`,
        `时间奖励 +${game.finalTime * 10}`,
        `金币 ×${game.coins} · 已提交到高分榜`,
      ];
  lines.forEach((ln, i) => ctx.fillText(ln, VIEW_W / 2, 140 + i * 22));

  ctx.font = "bold 16px monospace";
  ctx.fillStyle = "#ffcc00";
  ctx.fillText("按 ENTER 再来一局", VIEW_W / 2, 252);
}

// ============ 渲染主循环 ============
function loop(ts: number) {
  const dt = Math.min(ts - lastTime, 50) / 1000;
  lastTime = ts;
  // 固定 60fps 步进（简化：按帧更新）
  update();
  render();

  // 覆盖层
  if (game.state === "title") drawOverlayTitle();
  if (game.state === "clear") drawOverlayEnd(!game.won);

  rafId = requestAnimationFrame(loop);
  void dt;
}

// ============ 生命周期 ============
onMounted(() => {
  if (canvasRef.value) {
    const c = canvasRef.value.getContext("2d");
    if (c) ctx = c;
  }
  buildLevel();
  game.state = "title";
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("keyup", onKeyUp);
  rafId = requestAnimationFrame((t) => {
    lastTime = t;
    loop(t);
  });
});

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId);
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("keyup", onKeyUp);
});
</script>

<template>
  <div ref="rootRef" class="screen-root">
    <div class="scaled-stage" :style="stageStyle">
      <canvas
        ref="canvasRef"
        :width="VIEW_W"
        :height="VIEW_H"
        class="mario-canvas"
        tabindex="0"
      />
    </div>
    <div class="tip-row">
      <span>当前用户：{{ authStore.user?.username ?? "—" }}</span>
      <span class="keys">←→ 移动 · Space/↑ 跳跃 · X/C 火球 · R 重新开始 · Enter 开始</span>
    </div>
  </div>
</template>

<style scoped>
.screen-root {
  flex: 1;
  min-height: 0;
  width: 100%;
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: radial-gradient(circle at center, #1a2440 0%, #0a0e1a 80%);
  padding: 8px;
}
.scaled-stage {
  flex-shrink: 0;
  transform-origin: center center;
  line-height: 0;
}
.mario-canvas {
  width: 100%;
  height: 100%;
  display: block;
  background: #000;
  border: 3px solid #ffcc00;
  border-radius: 8px;
  image-rendering: pixelated;
  outline: none;
  box-shadow: 0 0 40px rgba(255, 204, 0, 0.25);
}
.tip-row {
  position: absolute;
  bottom: 10px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: center;
  gap: 24px;
  font-size: 13px;
  color: #aaa;
  padding: 0 16px;
  flex-wrap: wrap;
}
.keys {
  color: #ffcc00;
}
</style>