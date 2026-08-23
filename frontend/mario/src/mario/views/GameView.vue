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
  theme: "overworld" as "overworld" | "underground" | "sky" | "castle", // 当前关卡主题（背景/氛围）
  banner: "", // 关卡切换横幅文案
  bannerT: 0, // 横幅剩余帧数
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
  kind: "goomba" | "koopa" | "bowser"; // 栗子怪 / 乌龟 / 库巴(BOSS)
  state: "walk" | "shell" | "slide"; // 乌龟状态：行走 / 静止壳 / 滑行壳
  hp: number; // 生命值（普通敌人 1；库巴 >1）
  t: number; // 计时器（库巴跳跃节奏/受击闪烁）
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

// ============ 关卡生成（复刻经典马里奥多关卡：1-1 地上 / 1-2 地下 / 1-3 空中） ============
const LEVEL_COUNT = 4;

/** 生成一张全空网格 */
function newGrid(): TileChar[][] {
  const g: TileChar[][] = [];
  for (let r = 0; r < ROWS; r++) g.push(new Array<TileChar>(game.cols).fill(" "));
  return g;
}

/** 在网格某行铺一段瓦片 */
function putRow(grid: TileChar[][], row: number, from: number, to: number, t: TileChar) {
  for (let c = from; c <= to; c++) if (c >= 0 && c < game.cols) grid[row][c] = t;
}

/** 造一座逐级升高的阶梯（1-1 经典的“台阶”） */
function stairs(grid: TileChar[][], fromCol: number, height: number, t: TileChar = "X") {
  for (let s = 0; s < height; s++)
    for (let k = 0; k <= s; k++) grid[ROWS - 2 - k][fromCol + s] = t;
}

/** 清空动态对象（道具/火球/粒子） */
function resetDynamics() {
  game.items = [];
  game.fireballs = [];
  game.particles = [];
}

/** 按列放置敌人 */
function spawnEnemies(defs: Array<[number, "goomba" | "koopa"]>) {
  game.enemies = defs.map(([c, kind]) => ({
    x: c * TILE,
    y: groundTop() - 14,
    w: 14,
    h: 14,
    vx: kind === "koopa" ? -(0.3 + Math.random() * 0.3) : -(0.5 + Math.random() * 0.5),
    vy: 0,
    alive: true,
    kind,
    state: "walk" as "walk" | "shell" | "slide",
    hp: 1,
    t: 0,
  }));
}

/** 按当前 game.level 选择关卡 */
function buildLevel() {
  if (game.level === 2) buildLevel2();
  else if (game.level === 3) buildLevel3();
  else if (game.level === 4) buildLevel4();
  else buildLevel1();
}

// ---------------- 1-1 地上（地上世界） ----------------
function buildLevel1() {
  const grid = newGrid();
  game.theme = "overworld";

  // 地面（最底行）：含 1-1 标志性的 3 处坑洞（中坑/大坑/末段窄坑）
  for (let c = 0; c < game.cols; c++) {
    const pit =
      (c >= 63 && c <= 65) || // 中坑（3 格）
      (c >= 98 && c <= 102) || // 大坑（5 格，须助跑跳）
      (c >= 121 && c <= 122); // 末段窄坑（2 格）
    grid[ROWS - 1][c] = pit ? " " : "X";
  }

  // 主砖层（第 14 行）：砖与问号块交替，节奏贴近 1-1 前中段
  const blocks: Array<[number, TileChar]> = [
    [14, "B"], [16, "Q"], [17, "Q"], [18, "Q"], [20, "B"], [21, "B"],
    [23, "Q"], [24, "Q"], [25, "Q"], [28, "B"], [29, "B"],
    [32, "Q"], [33, "Q"], [34, "Q"], [35, "Q"],
    [38, "B"], [39, "B"], [42, "B"], [43, "B"], [44, "B"],
    [47, "Q"], [48, "Q"], [49, "Q"], // 空中台阶：跳上可登顶吃高金币
    [52, "B"], [53, "B"], [55, "Q"], [56, "Q"],
    [60, "B"], [61, "B"], [67, "B"], [68, "B"], [69, "B"],
    [74, "Q"], [75, "Q"], [76, "Q"],
    [79, "B"], [80, "B"], [81, "B"],
    [85, "Q"], [86, "Q"], [89, "B"], [90, "B"],
    [93, "Q"], [94, "Q"], [95, "Q"], [96, "Q"],
    [103, "B"], [104, "B"],
    [108, "Q"], [109, "Q"], [110, "Q"],
    [114, "B"], [115, "B"], [116, "B"],
    [120, "Q"], [123, "B"], [124, "Q"], [125, "Q"],
    [129, "B"], [130, "B"], [131, "B"],
    [133, "Q"], [134, "Q"], [135, "Q"], [136, "Q"],
    [141, "B"], [142, "B"], [145, "B"],
  ];
  for (const [c, t] of blocks) grid[14][c] = t;

  // 空中高台（第 7 行）+ 顶部悬挂金币
  const upper: Array<[number, number]> = [
    [47, 2], [63, 3], [76, 4], [103, 2], [114, 3], [131, 2], [143, 3],
  ];
  for (const [from, count] of upper) putRow(grid, 7, from, from + count - 1, "B");
  for (const c of [25, 26, 27, 49, 50, 64, 65, 66, 78, 79, 80, 81, 105, 106, 116, 117, 118, 133, 134, 145, 146, 147]) {
    grid[6][c] = "o";
  }

  // 主砖层下沿装饰金币线（跳起可吃）
  for (const c of [17, 18, 24, 25, 33, 34, 43, 44, 48, 49, 56, 57, 61, 62, 75, 76, 80, 81, 86, 87, 94, 95, 109, 110, 115, 116, 120, 121, 134, 135]) {
    grid[13][c] = "o";
  }

  // 水管：1-1 式高低错落，共 7 座
  const pipes: Array<[number, number]> = [
    [19, 2], [37, 3], [58, 3], [70, 3], [84, 4], [108, 2], [127, 3],
  ];
  for (const [c, h] of pipes) {
    for (let r = ROWS - 1 - h; r < ROWS - 1; r++) grid[r][c] = "P";
  }

  // 坑洞上方引导金币（提示助跑起跳时机）
  const pitCoins: Array<[number, number]> = [
    [64, 17], [65, 15], [66, 12],
    [100, 17], [101, 14], [102, 10],
    [118, 17], [119, 15],
  ];
  for (const [c, r] of pitCoins) grid[r][c] = "o";

  // 隐藏砖（H，顶到才显现，可借力登高）
  const hiddenCols = [31, 46, 62, 92, 107, 122, 137];
  for (const c of hiddenCols) grid[14][c] = "H";

  // 隐藏阶梯 + 顶端 1UP 问号块（经典彩蛋）
  {
    const col = 122;
    for (const r of [14, 11, 8, 5]) grid[r][col] = "H"; // 竖直阶梯（逐级差 3 格）
    grid[4][col] = "Q"; // 顶端问号块 → 顶出 1UP 绿蘑菇
    game.oneupBlocks = new Set([col]);
  }

  // 终点：渐升阶梯（两级）+ 旗杆 + 城堡
  stairs(grid, 143, 3);
  stairs(grid, 149, 4);
  game.flagCol = 160;
  for (let r = ROWS - 11; r <= ROWS - 2; r++) grid[r][160] = "P";
  grid[14][164] = "X";
  grid[13][165] = "X";
  grid[12][166] = "X";
  for (let r = ROWS - 6; r < ROWS; r++) grid[r][168] = "K";
  grid[ROWS - 8][169] = "K";
  grid[ROWS - 8][170] = "K";
  for (let r = ROWS - 6; r < ROWS; r++) grid[r][171] = "K";

  game.grid = grid;
  game.mushroomBlocks = new Set([16, 23, 55, 74, 93, 120]);
  game.flowerBlocks = new Set([17, 24, 47, 75, 108, 124]);
  game.coinBricks = new Set([28, 42, 60, 79, 103, 114, 129, 141]);

  spawnEnemies([
    [8, "goomba"], [12, "goomba"], [20, "koopa"], [27, "goomba"],
    [40, "koopa"], [46, "goomba"], [50, "goomba"], [55, "koopa"],
    [62, "goomba"], [67, "goomba"], [74, "koopa"], [83, "goomba"],
    [88, "goomba"], [93, "koopa"], [96, "goomba"], [105, "goomba"],
    [112, "koopa"], [118, "goomba"], [126, "koopa"], [132, "goomba"],
    [138, "goomba"], [144, "koopa"], [155, "goomba"], [156, "goomba"],
  ]);
  resetDynamics();
}

// ---------------- 1-2 地下（地下世界） ----------------
function buildLevel2() {
  const g = newGrid();
  game.theme = "underground";

  // 天花板（顶部两行砖，地下氛围）
  putRow(g, 0, 0, game.cols - 1, "X");
  putRow(g, 1, 0, game.cols - 1, "X");

  // 地面：全 X，含一大坑（70-74）与一小缺（120-121）
  for (let c = 0; c < game.cols; c++) {
    const pit = (c >= 70 && c <= 74) || (c >= 120 && c <= 121);
    g[ROWS - 1][c] = pit ? " " : "X";
  }

  // 地面上 1~2 高的砖墙堆（B），地下掩体障碍
  const walls: Array<[number, number]> = [
    [18, 2], [26, 1], [34, 2], [45, 2], [58, 1], [62, 2],
    [78, 2], [92, 1], [104, 2], [110, 2], [126, 1], [140, 2],
  ];
  for (const [c, h] of walls) for (let r = ROWS - 1 - h; r < ROWS - 1; r++) g[r][c] = "B";

  // 中高问号块与砖层（第 9 行）
  const ups: Array<[number, TileChar]> = [
    [22, "Q"], [23, "Q"], [24, "Q"], [30, "B"], [31, "B"],
    [38, "Q"], [39, "Q"], [49, "Q"], [50, "Q"], [53, "B"], [54, "B"],
    [64, "B"], [65, "B"], [68, "Q"], [69, "Q"],
    [80, "B"], [81, "B"], [86, "Q"], [87, "Q"], [88, "Q"],
    [96, "B"], [97, "B"], [102, "Q"], [103, "Q"],
    [112, "B"], [113, "B"], [118, "Q"], [119, "Q"], [128, "B"], [129, "B"],
    [132, "Q"], [133, "Q"], [134, "Q"], [138, "B"], [139, "B"],
    [142, "Q"], [143, "Q"], [144, "Q"],
  ];
  for (const [c, t] of ups) g[9][c] = t;

  // 金币：墙顶一列 + 坑上弧线 + 大坑后收尾
  for (const c of [25, 26, 27, 35, 36, 46, 47, 63, 64, 82, 83, 95, 96, 105, 106, 115, 116, 127, 128, 135, 136]) g[14][c] = "o";
  const pitCoins: Array<[number, number]> = [
    [72, 17], [73, 14], [74, 10], [116, 17], [117, 15],
  ];
  for (const [c, r] of pitCoins) g[r][c] = "o";

  // 隐藏砖 + 中段 1UP 问号块
  for (const c of [21, 44, 60, 90, 107, 131]) g[9][c] = "H";
  game.oneupBlocks = new Set([68]);

  // 敌人（地下多栗子怪与乌龟）
  spawnEnemies([
    [14, "goomba"], [20, "goomba"], [28, "koopa"], [40, "goomba"],
    [52, "koopa"], [60, "goomba"], [66, "goomba"], [82, "koopa"],
    [90, "goomba"], [96, "goomba"], [104, "koopa"], [114, "goomba"],
    [124, "koopa"], [130, "goomba"], [138, "koopa"], [146, "goomba"],
  ]);

  // 终点旗杆 + 城堡
  game.flagCol = 155;
  for (let r = ROWS - 11; r <= ROWS - 2; r++) g[r][155] = "P";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][163] = "K";
  g[ROWS - 8][164] = "K";
  g[ROWS - 8][165] = "K";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][166] = "K";

  game.grid = g;
  game.mushroomBlocks = new Set([22, 49, 86, 118]);
  game.flowerBlocks = new Set([23, 50, 102, 142]);
  game.coinBricks = new Set();
  resetDynamics();
}

// ---------------- 1-3 空中（云端世界） ----------------
function buildLevel3() {
  const g = newGrid();
  game.theme = "sky";

  // 空中地面为实心砖（X，稳妥落脚），多处小缺口（掉空即亡）
  for (let c = 0; c < game.cols; c++) {
    const gap =
      (c >= 16 && c <= 17) || (c >= 40 && c <= 42) || (c >= 66 && c <= 68) ||
      (c >= 92 && c <= 94) || (c >= 118 && c <= 119) || (c >= 142 && c <= 144);
    g[ROWS - 1][c] = gap ? " " : "X";
  }

  // 高层砖台（第 7 行）多段
  const upper: Array<[number, number]> = [
    [26, 3], [48, 3], [70, 4], [100, 3], [122, 4], [148, 3],
  ];
  for (const [from, count] of upper) putRow(g, 7, from, from + count - 1, "B");

  // 空中砖块与问号块（第 11 行）
  const mid: Array<[number, TileChar]> = [
    [10, "B"], [12, "Q"], [14, "B"], [20, "Q"], [24, "B"],
    [31, "Q"], [34, "B"], [45, "Q"], [52, "B"], [54, "Q"], [58, "B"],
    [63, "Q"], [76, "B"], [78, "Q"], [81, "B"], [87, "Q"], [99, "B"],
    [105, "Q"], [110, "B"], [113, "Q"], [126, "B"], [129, "Q"], [134, "B"],
    [139, "Q"], [149, "B"], [151, "Q"], [153, "B"],
  ];
  for (const [c, t] of mid) g[11][c] = t;

  // 金币弧线（起跳引导）与平台金币
  const arcs: Array<[number, number]> = [
    [17, 16], [18, 14], [19, 12], [41, 15], [42, 13], [43, 11],
    [67, 16], [68, 14], [69, 12], [93, 15], [94, 13], [95, 11],
    [119, 16], [120, 14], [143, 15], [144, 13], [145, 11],
  ];
  for (const [c, r] of arcs) g[r][c] = "o";

  // 敌人（空中多乌龟 + 栗子怪）
  spawnEnemies([
    [10, "goomba"], [15, "koopa"], [23, "goomba"], [30, "koopa"],
    [38, "goomba"], [47, "koopa"], [56, "goomba"], [64, "koopa"],
    [75, "goomba"], [84, "koopa"], [97, "goomba"], [108, "koopa"],
    [120, "goomba"], [131, "koopa"], [147, "goomba"], [150, "koopa"],
  ]);

  // 终点旗杆 + 城堡
  game.flagCol = 158;
  for (let r = ROWS - 11; r <= ROWS - 2; r++) g[r][158] = "P";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][166] = "K";
  g[ROWS - 8][167] = "K";
  g[ROWS - 8][168] = "K";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][169] = "K";

  game.grid = g;
  game.mushroomBlocks = new Set([12, 54, 105, 129]);
  game.flowerBlocks = new Set([20, 78]);
  game.coinBricks = new Set();
  game.oneupBlocks = new Set([87]);
  resetDynamics();
}

// ---------------- 1-4 城堡（BOSS 关：库巴） ----------------
function buildLevel4() {
  const g = newGrid();
  game.theme = "castle";

  // 天花板（顶部两行砖，城堡内部氛围）
  putRow(g, 0, 0, game.cols - 1, "X");
  putRow(g, 1, 0, game.cols - 1, "X");

  // 地面：全 X（无坑，专注 BOSS 战）+ 岩浆坑装饰（仅末段画 L？用普通 X，保持可玩）
  for (let c = 0; c < game.cols; c++) g[ROWS - 1][c] = "X";

  // 空中砖台（第 7 行）
  const plat: Array<[number, number]> = [[28, 6], [58, 5], [88, 6], [112, 5]];
  for (const [from, count] of plat) putRow(g, 7, from, from + count - 1, "B");

  // 砖 / 问号层（第 10 行）
  const ups: Array<[number, TileChar]> = [
    [16, "Q"], [17, "Q"], [20, "B"], [21, "B"],
    [34, "Q"], [35, "Q"], [40, "B"], [42, "B"],
    [52, "Q"], [55, "Q"], [62, "B"], [66, "B"],
    [74, "Q"], [78, "Q"], [80, "B"], [84, "B"],
    [94, "Q"], [97, "Q"], [102, "B"], [104, "B"],
    [114, "Q"], [117, "Q"], [120, "B"], [123, "B"],
  ];
  for (const [c, t] of ups) g[10][c] = t;

  // 金币
  for (const c of [23, 24, 25, 36, 37, 45, 46, 68, 69, 82, 83, 99, 100, 107, 108, 121, 122]) g[13][c] = "o";
  for (const c of [30, 31, 32, 60, 61, 62, 90, 91, 92, 114, 115, 116]) g[6][c] = "o";

  // 敌人（前中段，为 BOSS 铺路而设）
  spawnEnemies([
    [12, "goomba"], [18, "goomba"], [24, "koopa"], [38, "goomba"], [44, "koopa"],
    [54, "goomba"], [60, "koopa"], [72, "goomba"], [78, "goomba"], [86, "koopa"],
    [96, "goomba"], [100, "goomba"], [108, "koopa"], [116, "goomba"],
  ]);

  // ---- BOSS 竞技场：两侧砖墙（2 高，可跳过进出）围成的平地，库巴在其中巡逻 ----
  for (let r = ROWS - 3; r <= ROWS - 2; r++) { g[r][127] = "X"; g[r][149] = "X"; }
  // 库巴（BOSS）：体型大，hp 5，受 5 击或 5 踩灭；在墙内往返巡逻并偶尔起跳
  game.enemies.push({
    x: 138 * TILE,
    y: groundTop() - 32,
    w: 30,
    h: 32,
    vx: -0.7,
    vy: 0,
    alive: true,
    kind: "bowser",
    state: "walk",
    hp: 5,
    t: 0,
  });

  game.grid = g;
  game.mushroomBlocks = new Set([16, 34, 52, 74, 94, 114]);
  game.flowerBlocks = new Set([17, 35, 55, 78, 97, 117]);
  game.coinBricks = new Set([20, 42, 66, 84, 104, 123]);
  game.oneupBlocks = new Set();

  // 终点旗杆 + 城堡（越过库巴竞技场后）
  game.flagCol = 160;
  for (let r = ROWS - 11; r <= ROWS - 2; r++) g[r][160] = "P";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][168] = "K";
  g[ROWS - 8][169] = "K";
  g[ROWS - 8][170] = "K";
  for (let r = ROWS - 6; r < ROWS; r++) g[r][171] = "K";
  resetDynamics();
}

// ============ 游戏流程 ============
function newGame() {
  game.level = 1;
  game.theme = "overworld";
  buildLevel();
  game.state = "play";
  game.score = 0;
  game.coins = 0;
  game.lives = 3;
  game.time = 200;
  game.dead = false;
  game.won = false;
  game.fireballs = [];
  game.fireCd = 0;
  game.banner = "";
  game.bannerT = 0;
  spawnMario();
  setBanner("WORLD 1-1");
}

/** 弹出关卡切换横幅（约 2 秒） */
function setBanner(text: string) {
  game.banner = text;
  game.bannerT = 120;
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

/** 通关（含跨关推进）：非最后关进入下一关，最后关结算提交成绩 */
function doWin(heightBonus: number) {
  game.finalTime = game.time;
  game.flagBonus = heightBonus;
  const timeBonus = game.time * 10;
  game.score += heightBonus + timeBonus;

  // 还有下一关：保留得分/金币/生命，仅切换地图、主题与出生点
  if (game.level < LEVEL_COUNT) {
    game.level += 1;
    game.time = 250;
    buildLevel();
    spawnMario();
    game.state = "play";
    setBanner(`WORLD 1-${game.level}`);
    return;
  }

  // 通关全部关卡：结算并提交成绩
  game.state = "clear";
  game.won = true;
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
  if (game.bannerT > 0) game.bannerT--; // 关卡横幅倒计时

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
          if (e.kind === "bowser") {
            // 库巴：火球只掉 1 点血（1 击扣 1 血，5 血击倒），受击闪烁
            e.hp -= 1;
            e.t = 18;
            if (e.hp <= 0) {
              e.alive = false;
              game.score += 5000;
            }
          } else {
            e.alive = false;
            game.score += 200;
          }
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

      // 库巴（BOSS）：受击闪烁倒计时 + 落地时偶尔起跳
      if (e.kind === "bowser") {
        if (e.t > 0) e.t--;
        if (e.vy === 0 && game.frame % 170 === 0) e.vy = -9.5; // 落地时偶尔起跳
      }

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
          if (e.kind === "bowser") {
            // 库巴：踩踏扣 1 血（5 血击倒），受击闪烁；未死则弹开
            e.hp -= 1;
            e.t = 18;
            if (e.hp <= 0) {
              e.alive = false;
              game.score += 5000;
            }
          } else if (e.kind === "goomba") {
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

  // 天（按关卡主题配色）
  const sky = ctx.createLinearGradient(0, 0, 0, VIEW_H);
  if (game.theme === "underground") {
    // 地下：暗紫蓝渐变
    sky.addColorStop(0, "#16162a");
    sky.addColorStop(1, "#3a2a4a");
  } else if (game.theme === "sky") {
    // 空中：深蓝高空渐变
    sky.addColorStop(0, "#1c3aa8");
    sky.addColorStop(1, "#6fc4ff");
  } else if (game.theme === "castle") {
    // 城堡（BOSS）：暗红/岩浆般氛围
    sky.addColorStop(0, "#3a0f0f");
    sky.addColorStop(1, "#7a2418");
  } else {
    // 地上：浅蓝天空
    sky.addColorStop(0, "#6fb8ff");
    sky.addColorStop(1, "#bde3ff");
  }
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, VIEW_W, VIEW_H);

  // 云（地下 / 城堡主题不画）
  if (game.theme !== "underground" && game.theme !== "castle") {
    ctx.fillStyle = "rgba(255,255,255,0.9)";
    drawCloud(40, 55, 26);
    drawCloud(200, 90, 22);
    drawCloud(360, 45, 18);
    drawCloud(150, 130, 30);
    drawCloud(280, 150, 24);
  }

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

  // 关卡切换横幅（跨关/开局时短暂显示）
  if (game.bannerT > 0 && game.banner) {
    ctx.fillStyle = "rgba(0,0,0,0.55)";
    const bh = 44;
    const by = VIEW_H / 2 - bh / 2;
    ctx.fillRect(10, by, VIEW_W - 20, bh);
    ctx.textAlign = "center";
    ctx.fillStyle = "#ffcc00";
    ctx.font = "bold 26px monospace";
    ctx.fillText(game.banner, VIEW_W / 2, by + 30);
  }
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
  if (e.kind === "bowser") {
    // ===== 库巴（BOSS）=====
    const flash = e.t > 0 && Math.floor(e.t / 4) % 2 === 0;
    ctx.globalAlpha = flash ? 0.4 : 1;
    const ex = px + (e.vx > 0 ? 1 : -1);
    // 身体（绿色）
    ctx.fillStyle = "#3f9e33";
    ctx.fillRect(ex, e.y + 6, e.w, e.h - 8);
    // 顶部尖刺壳
    ctx.fillStyle = "#7a4a12";
    ctx.beginPath();
    ctx.moveTo(ex + 2, e.y + 6);
    ctx.lineTo(ex + 8, e.y + 1);
    ctx.lineTo(ex + 14, e.y + 6);
    ctx.fill();
    ctx.beginPath();
    ctx.moveTo(ex + 14, e.y + 6);
    ctx.lineTo(ex + 20, e.y + 1);
    ctx.lineTo(ex + 26, e.y + 6);
    ctx.fill();
    // 头（橄榄）
    ctx.fillStyle = "#6aa84f";
    ctx.fillRect(ex + (e.vx > 0 ? -3 : e.w - 3), e.y + 4, 4, 6);
    // 眼睛
    ctx.fillStyle = "#fff";
    ctx.fillRect(ex + (e.vx > 0 ? -2 : e.w - 2), e.y + 3, 3, 3);
    ctx.fillStyle = "#111";
    ctx.fillRect(ex + (e.vx > 0 ? 0 : e.w), e.y + 4, 2, 2);
    // 腹（浅）
    ctx.fillStyle = "#c8e0b0";
    ctx.fillRect(ex + 3, e.y + e.h - 10, e.w - 6, 6);
    // 脚
    ctx.fillStyle = "#5a7a2a";
    ctx.fillRect(ex + 2, e.y + e.h - 4, 8, 4);
    ctx.fillRect(ex + e.w - 10, e.y + e.h - 4, 8, 4);
    ctx.globalAlpha = 1;
    return;
  }
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