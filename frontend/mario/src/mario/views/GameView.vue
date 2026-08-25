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
import { onBeforeUnmount, onMounted, reactive, ref } from "vue";
import * as THREE from "three";
import { useAuthStore } from "@/stores/auth";
import { marioApi } from "@/api";

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

const authStore = useAuthStore();

// 注：不再用 CSS transform:scale 固定画布——3D 渲染直接适配容器实际分辨率（见 initThree/resize），
// 任意屏幕原生清晰，无需等比缩放放大插值。

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
  levelVersion: 0, // 关卡重建计数（buildLevel 递增，三维渲染据此清空旧瓦片）
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
  game.levelVersion++; // 通知三维渲染层清空并重建瓦片
}

// ---------------- 1-1 地上（地上世界） ----------------
function buildLevel1() {
  const grid = newGrid();
  game.theme = "overworld";

  // 地面（最底行）：含 1-1 标志性的 3 处坑洞（中坑/大坑/末段窄坑）
  // 坑宽以“小号马里奥最远平跳约 70px≈4.4 格”为准，最宽设为 4 格，
  // 否则小马里奥助跑也无法越过（之前 5 格大坑=必经 bug，跳不过去）。
  for (let c = 0; c < game.cols; c++) {
    const pit =
      (c >= 63 && c <= 65) || // 中坑（3 格）
      (c >= 98 && c <= 101) || // 大坑（4 格，助跑可跳）
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

  // 水管：1-1 式高低错落，共 7 座。
  // 最高 3 格（48px）：小号马里奥最大跳高 64px，4 格（64px）会顶满跳不过，
  // 故原 [84,4] 降至 3 格，保证小号也能跳过/登上。
  const pipes: Array<[number, number]> = [
    [19, 2], [37, 3], [58, 3], [70, 3], [84, 3], [108, 2], [127, 3],
  ];
  for (const [c, h] of pipes) {
    for (let r = ROWS - 1 - h; r < ROWS - 1; r++) grid[r][c] = "P";
  }

  // 坑洞上方引导金币（提示助跑起跳时机）
  const pitCoins: Array<[number, number]> = [
    [64, 17], [65, 15], [66, 12],
    [99, 15], [100, 11], [101, 15],
    [118, 17], [119, 15],
  ];
  for (const [c, r] of pitCoins) grid[r][c] = "o";

  // 隐藏砖（H，顶到才显现，可借力登高）
  // 注意：不要放在坑洞正上方（脚下是坑、无法从下方点亮，只会当隐形墙挡跳跃）。
  const hiddenCols = [31, 46, 62, 92, 107, 137];
  for (const c of hiddenCols) grid[14][c] = "H";

  // 隐藏阶梯 + 顶端 1UP 问号块（经典彩蛋）
  // 置于 c92 平地上（可站在下方逐级点亮），原 c122 正在 121-122 窄坑上方，故迁移。
  {
    const col = 92;
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

  // 地面：全 X，含一大坑（70-73，4 格，小马里奥助跑可跳）与一小缺（120-121）
  for (let c = 0; c < game.cols; c++) {
    const pit = (c >= 70 && c <= 73) || (c >= 120 && c <= 121);
    g[ROWS - 1][c] = pit ? " " : "X";
  }

  // 地面上 1~2 高的砖墙堆（B），地下掩体障碍
  const walls: Array<[number, number]> = [
    [18, 2], [26, 1], [34, 2], [45, 2], [58, 1], [62, 2],
    [78, 2], [92, 1], [104, 2], [110, 2], [126, 1], [140, 2],
  ];
  for (const [c, h] of walls) for (let r = ROWS - 1 - h; r < ROWS - 1; r++) g[r][c] = "B";

  // 可顶问号块与砖层（第 14 行）
  // 原放第 9 行（下沿高达 160px，小号马里奥头顶最高 220px 够不到，连大号也顶不到），
  // 下移到第 14 行（与第 1 关一致），小号只需跳到约 44px 即可顶到。
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
  for (const [c, t] of ups) g[14][c] = t;

  // 金币：墙顶一列 + 坑上弧线 + 大坑后收尾（放第 13 行，避开第 14 行的可顶层）
  for (const c of [25, 26, 27, 35, 36, 46, 47, 63, 64, 82, 83, 95, 96, 105, 106, 115, 116, 127, 128, 135, 136]) g[13][c] = "o";
  const pitCoins: Array<[number, number]> = [
    [71, 15], [72, 12], [73, 15], [116, 17], [117, 15],
  ];
  for (const [c, r] of pitCoins) g[r][c] = "o";

  // 隐藏砖 + 中段 1UP 问号块（放第 13 行，小号可顶到；1UP 的 Q 在第 14 行可顶层）
  for (const c of [21, 44, 60, 90, 107, 131]) g[13][c] = "H";
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

  // 空中砖块与问号块（第 14 行）
  // 原放第 11 行（下沿 192px，小号马里奥头顶最高 220px 够不到），下移到第 14 行可顶。
  const mid: Array<[number, TileChar]> = [
    [10, "B"], [12, "Q"], [14, "B"], [20, "Q"], [24, "B"],
    [31, "Q"], [34, "B"], [45, "Q"], [52, "B"], [54, "Q"], [58, "B"],
    [63, "Q"], [76, "B"], [78, "Q"], [81, "B"], [87, "Q"], [99, "B"],
    [105, "Q"], [110, "B"], [113, "Q"], [126, "B"], [129, "Q"], [134, "B"],
    [139, "Q"], [149, "B"], [151, "Q"], [153, "B"],
  ];
  for (const [c, t] of mid) g[14][c] = t;

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

  // 砖 / 问号层（第 14 行）
  // 原放第 10 行（下沿 176px，小号马里奥头顶最高 220px 够不到，连大号也顶不到），
  // 下移到第 14 行可顶（金币在第 13 行，不同行不冲突）。
  const ups: Array<[number, TileChar]> = [
    [16, "Q"], [17, "Q"], [20, "B"], [21, "B"],
    [34, "Q"], [35, "Q"], [40, "B"], [42, "B"],
    [52, "Q"], [55, "Q"], [62, "B"], [66, "B"],
    [74, "Q"], [78, "Q"], [80, "B"], [84, "B"],
    [94, "Q"], [97, "Q"], [102, "B"], [104, "B"],
    [114, "Q"], [117, "Q"], [120, "B"], [123, "B"],
  ];
  for (const [c, t] of ups) g[14][c] = t;

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
  submitResult();
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
    // 游戏结束：把本局成绩也提交到高分榜（失败局同样上榜，否则排行榜会长期空白）
    game.state = "clear";
    game.won = false;
    submitResult();
  }
}

/** 把本局成绩提交到高分榜（有分才提交；通关/结束时各调用一次） */
function submitResult() {
  if (game.score <= 0) return; // 零分不占榜
  void marioApi
    .submitScore({
      score: game.score,
      level: game.level,
      coins: game.coins,
      time_ms: 0,
    })
    .catch(() => undefined);
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
  } else if (game.state === "play" && (k === "r" || k === "R")) {
    // R 重新开始当前局（从头再来）
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
      // 旗杆列是实心 "P"，水平碰撞会把马里奥右缘钳制在 pX1 - 0.01，
      // 因此不能用 `m.x + m.w > pX1`（永远不成立），改用“右缘已贴到旗杆列”
      // 的相邻判定（容忍钳制产生的小内缩）。
      const touchesFlagCol = m.x + m.w >= pX1 - 1 && m.x < pX2;
      if (touchesFlagCol && m.y + m.h > pTopY && m.y < pBotY) {
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

// ============ 三维渲染（Three.js 体素方块） ============
// 坐标映射：2D 游戏逻辑坐标(y 轴向下) → 3D 世界（X 水平前进、Y 高度向上、Z 纵深厚度）：
//   X = x2d，Y = VIEW_H - y2d，Z = 0（厚度围绕 Z 展开，体现体素立体感）。
// 游戏逻辑（物理/碰撞/关卡/得分）完全不变，仅渲染层由 2D Canvas 换成 3D 体素。

// —— 渲染宿主与 Three 核心对象 ——
const renderHost = ref<HTMLElement | null>(null);
let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let worldGroup: THREE.Group | null = null;
const bgColor = new THREE.Color(0x6fb8ff);

// 主题天空底色（偏明亮；并同步雾色以保持协调）
function setBackground(theme: string) {
  let c = 0x8fd0ff; // 地上：明亮天蓝
  if (theme === "underground") c = 0x3a3a6e; // 地下：提亮的暗紫蓝
  else if (theme === "sky") c = 0x4fb0e6; // 空中：明亮高空蓝
  else if (theme === "castle") c = 0x5a2020; // 城堡：提亮的暗红
  bgColor.set(c);
  if (scene?.fog) scene.fog.color.set(c);
}

// —— 共享几何与材质缓存（避免每帧新建） ——
const geoUnit = new THREE.BoxGeometry(1, 1, 1); // 经 scale 拉伸成部件
const geoCoin = new THREE.OctahedronGeometry(6, 0);
const geoFire = new THREE.SphereGeometry(5, 8, 6);
const boxGeos = new Map<string, THREE.BoxGeometry>();
function boxGeom(sx: number, sy: number, sz: number): THREE.BoxGeometry {
  const k = sx + "_" + sy + "_" + sz;
  let g = boxGeos.get(k);
  if (!g) {
    g = new THREE.BoxGeometry(sx, sy, sz);
    boxGeos.set(k, g);
  }
  return g;
}
// 默认用卡通材质（MeshToonMaterial）：明暗对比鲜明、色彩亮丽，配合强光照更好看
const matCache = new Map<number, THREE.MeshToonMaterial>();
function mat(color: number): THREE.MeshToonMaterial {
  let m = matCache.get(color);
  if (!m) {
    m = new THREE.MeshToonMaterial({ color });
    matCache.set(color, m);
  }
  return m;
}

/** 向父组添加一个用单位格子拉伸的体素部件（局部坐标原点=实体中心） */
function addPart(p: THREE.Group, x: number, y: number, z: number, sx: number, sy: number, sz: number, color: number) {
  const mesh = new THREE.Mesh(geoUnit, mat(color));
  mesh.scale.set(sx, sy, sz);
  mesh.position.set(x, y, z);
  p.add(mesh);
}
/** 清空组内的部件网格（几何与材质共享，无需 dispose） */
function clearGroup(p: THREE.Group) {
  for (const ch of p.children) p.remove(ch);
}

// —— 瓦片（grid → 3D 方块/金币） ——
const TILE_DEPTH = 13; // 方块厚度（约为 1 个 TILE 的 0.8，体现体素立体感）
const tileMesh = new Map<number, THREE.Group>();
const tileChar = new Map<number, string>();
const TILE_COLORS: Record<string, number> = {
  X: 0xd96a2b, P: 0xd96a2b, B: 0xc95a20, Q: 0xffb400, "#": 0xb07a34, K: 0xcfcfcf,
};

// —— 3D 瓦片建模（体素块 + 顶部高光/细节，比纯单色块更好看） ——
/** 顶部薄高光（让砖块/地面有受光层次） */
function topSlab(color: number): THREE.Mesh {
  const m = new THREE.Mesh(boxGeom(TILE, 2, TILE_DEPTH), mat(color));
  m.position.set(0, TILE / 2 - 1, 0);
  return m;
}
let qTex: THREE.Texture | null = null;
/** 问号块上的白色“?”（Sprite 始终面向相机） */
function makeQMark(): THREE.Sprite {
  if (!qTex) {
    const c = document.createElement("canvas");
    c.width = c.height = 128;
    const g2 = c.getContext("2d")!;
    g2.clearRect(0, 0, 128, 128);
    g2.fillStyle = "#ffffff";
    g2.font = "bold 96px Impact, sans-serif";
    g2.textAlign = "center";
    g2.textBaseline = "middle";
    g2.fillText("?", 64, 66);
    qTex = new THREE.CanvasTexture(c);
  }
  const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: qTex, transparent: true, depthWrite: false }));
  sp.scale.set(11, 11, 1);
  sp.position.set(0, 0, TILE_DEPTH / 2 + 1);
  return sp;
}
/** 按瓦片类型构建一组体素块 */
function makeTile(t: TileChar): THREE.Group {
  const g = new THREE.Group();
  if (t === "o") {
    g.add(new THREE.Mesh(geoCoin, mat(0xffce2e)));
    g.userData.coin = true;
    return g;
  }
  const color = TILE_COLORS[t] ?? 0xffffff;
  g.add(new THREE.Mesh(boxGeom(TILE, TILE, TILE_DEPTH), mat(color)));
  if (t === "B") {
    // 砖块：顶部亮条 + 底部暗边，立体砖纹
    g.add(topSlab(0xffd9a0));
    const bot = new THREE.Mesh(boxGeom(TILE, 3, TILE_DEPTH), mat(0x8a3a10));
    bot.position.set(0, -TILE / 2 + 1.5, 0);
    g.add(bot);
  } else if (t === "X" || t === "P") {
    // 地面 / 水管：顶部亮边
    g.add(topSlab(0xffe3b0));
  } else if (t === "Q") {
    // 问号块：白“?”浮标
    g.add(makeQMark());
  } else if (t === "#") {
    // 已使用块：顶部压暗，区别于未使用
    const dark = new THREE.Mesh(boxGeom(TILE, 2, TILE_DEPTH), mat(0x6e4a18));
    dark.position.set(0, TILE / 2 - 1, 0);
    g.add(dark);
  }
  return g;
}
// 关卡重载（buildLevel 触发）时清空旧瓦片
let lastLevelVersion = -1;
function clearTileMeshes() {
  const w = worldGroup;
  if (!w) return;
  for (const m of tileMesh.values()) w.remove(m);
  tileMesh.clear();
  tileChar.clear();
}
function syncTiles() {
  const w = worldGroup;
  if (!w) return;
  if (lastLevelVersion !== game.levelVersion) {
    lastLevelVersion = game.levelVersion;
    clearTileMeshes();
  }
  // 按 3D 相机可见的水平范围建瓦片。以「相机的实际 x」为中心（它已包含关卡端部 clamp），
// 否则相机停在端部时（相机中心≠马里奥位置）右侧视野会缺瓦片，地面断裂错乱。
  const cx0 = camera?.position.x ?? game.mario.x + game.mario.w / 2;
  const cStart = Math.floor((cx0 - CAM_HALF_W - TILE) / TILE);
  const cEnd = Math.floor((cx0 + CAM_HALF_W + TILE) / TILE);
  for (let r = 0; r < ROWS; r++) {
    for (let c = cStart; c <= cEnd; c++) {
      if (c < 0 || c >= game.cols) continue;
      const t = game.grid[r][c];
      const key = r * 1000 + c;
      if (tileChar.get(key) === t) continue; // 未变化，跳过
      tileChar.set(key, t);
      const old = tileMesh.get(key);
      if (old) {
        w.remove(old);
        tileMesh.delete(key);
      }
      // 空格与隐藏砖(H)不渲染（H 仍参与 2D 碰撞，顶到变 '#' 后才会显示）
      if (t === " " || t === "H") continue;
      const x = c * TILE + TILE / 2;
      const y = VIEW_H - (r * TILE + TILE / 2);
      const grp = makeTile(t);
      grp.position.set(x, y, 0);
      w.add(grp);
      tileMesh.set(key, grp);
    }
  }
  // 金币轻微旋转（仅直接遍历可见金币网格）
  for (const m of tileMesh.values()) {
    if (m.userData.coin) m.rotation.z += 0.12;
  }
}

// —— 马里奥体素模型 ——
const marioModel = new THREE.Group();
let marioSize = -1; // 用于检测大小切换
function buildMarioInto(mario: THREE.Group, big: boolean) {
  const w = big ? 16 : 14;
  const h = big ? 32 : 20;
  const half = h / 2;
  const dep = 8;
  // 帽子
  addPart(mario, 0, half - 3, 0, w, 6, dep + 2, 0xe23b2e);
  // 帽檐
  addPart(mario, 0, half - 5, 0, w + 3, 2, dep + 3, 0xe23b2e);
  // 脸
  addPart(mario, 0, half - 12, 0, w - 1, 8, dep, 0xf7c59f);
  // 眼（朝右）
  addPart(mario, w / 2 - 3, half - 11, 0, 3, 3, dep + 2, 0x333333);
  // 髭
  addPart(mario, 0, half - 18, 0, w - 1, 2, dep, 0x7a3b1a);
  // 工装
  addPart(mario, 0, -half + (big ? 11 : 8), 0, w - 1, big ? 13 : 8, dep, 0x2a6bd8);
  // 脚（连体简化）
  addPart(mario, 0, -half + 2, 0, w - 1, 3, dep + 2, 0x8a4a24);
}
function syncMario() {
  const w = worldGroup;
  if (!w) return;
  const m = game.mario;
  if (marioSize !== m.size) {
    marioSize = m.size;
    clearGroup(marioModel);
    buildMarioInto(marioModel, m.size === 1);
  }
  // 无敌帧闪烁
  if (m.inv > 0 && Math.floor(m.inv / 4) % 2 === 0) marioModel.visible = false;
  else marioModel.visible = true;
  marioModel.position.set(m.x + m.w / 2, VIEW_H - (m.y + m.h / 2), 0);
  marioModel.scale.set(m.facing < 0 ? -1 : 1, 1, 1);
}

// —— 敌人体素模型（池化，按索引复用；kind 变化时重建组件） ——
const enemyModels: THREE.Group[] = [];
const enemyKinds: (string | null)[] = [];
function buildGoomba(g: THREE.Group) {
  addPart(g, 0, 0, 0, 14, 10, 8, 0xa03e1f); // 身体
  addPart(g, 0, 6, 0, 12, 8, 8, 0x6b2412); // 头
  addPart(g, -3, 7, 0, 3, 2, 9, 0xffffff); // 眼
  addPart(g, 3, 7, 0, 3, 2, 9, 0xffffff);
  addPart(g, 0, -6, 0, 14, 2, 8, 0x5a3318); // 脚
}
function buildKoopa(g: THREE.Group) {
  addPart(g, 0, 2, 0, 16, 12, 9, 0x3f9e33); // 壳
  addPart(g, 0, 9, 0, 10, 4, 9, 0xe8f2c0); // 壳顶
  addPart(g, 5, 3, -2, 3, 5, 8, 0x8a5a2a); // 头（偏右）
  addPart(g, 5, 4, -2, 2, 2, 9, 0x333333); // 眼
  addPart(g, 0, -5, 0, 16, 1, 9, 0x5a7a2a); // 脚
}
function buildBowser(g: THREE.Group) {
  addPart(g, 0, 0, 0, 30, 22, 16, 0x3f9e33); // 身体
  addPart(g, 0, 13, 0, 20, 10, 18, 0x7a4a12); // 尖刺壳
  addPart(g, -16, 3, 0, 7, 13, 12, 0x6aa84f); // 头（左）
  addPart(g, -14, 6, 0, 5, 5, 13, 0xffffff); // 眼
  addPart(g, 0, -12, 0, 30, 3, 18, 0x5a7a2a); // 脚
}
function syncEnemies() {
  const w = worldGroup;
  if (!w) return;
  const n = Math.max(enemyModels.length, game.enemies.length);
  for (let i = 0; i < n; i++) {
    const e = game.enemies[i];
    if (!e) {
      if (enemyModels[i]) enemyModels[i].visible = false;
      continue;
    }
    let grp = enemyModels[i];
    if (!grp) {
      grp = new THREE.Group();
      w.add(grp);
      enemyModels[i] = grp;
      enemyKinds[i] = null;
    }
    if (enemyKinds[i] !== e.kind) {
      enemyKinds[i] = e.kind;
      clearGroup(grp);
      if (e.kind === "goomba") buildGoomba(grp);
      else if (e.kind === "koopa") buildKoopa(grp);
      else buildBowser(grp);
    }
    grp.visible = e.kind !== "bowser" || e.t <= 0 || Math.floor(e.t / 4) % 2 !== 0;
    grp.position.set(e.x + e.w / 2, VIEW_H - (e.y + e.h / 2), 0);
    grp.scale.set(e.vx < 0 ? -1 : 1, 1, 1);
  }
}

// —— 道具体素模型（蘑菇/火花/1UP） ——
const itemModels: THREE.Group[] = [];
function buildMushroom(g: THREE.Group) {
  addPart(g, 0, 0, 0, 14, 8, 8, 0xfff6e0); // 柄
  addPart(g, 0, 5, 0, 14, 7, 8, 0xe23b2e); // 红帽
  addPart(g, -3, 5, 0, 3, 3, 10, 0xffffff); // 白点
  addPart(g, 2, 5, 0, 3, 3, 10, 0xffffff);
}
function buildFlower(g: THREE.Group) {
  addPart(g, 0, 0, 0, 3, 6, 3, 0x3f9e33); // 茎
  addPart(g, 0, 5, 0, 12, 5, 12, 0xffffff); // 花瓣
  addPart(g, 0, 7, 0, 6, 3, 9, 0xffcf3a); // 芯
}
function buildOneUp(g: THREE.Group) {
  addPart(g, 0, 0, 0, 14, 8, 8, 0xfff6e0); // 柄
  addPart(g, 0, 5, 0, 14, 7, 8, 0x3f9e33); // 绿帽
  addPart(g, -2, 5, 0, 3, 3, 10, 0xffffff);
  addPart(g, 2, 5, 0, 3, 3, 10, 0xffffff);
}
function syncItems() {
  const w = worldGroup;
  if (!w) return;
  const n = Math.max(itemModels.length, game.items.length);
  for (let i = 0; i < n; i++) {
    const it = game.items[i];
    if (!it) {
      if (itemModels[i]) itemModels[i].visible = false;
      continue;
    }
    let grp = itemModels[i];
    if (!grp) {
      grp = new THREE.Group();
      w.add(grp);
      itemModels[i] = grp;
    }
    grp.visible = true;
    grp.rotation.y += 0.05;
    grp.position.set(it.x + it.w / 2, VIEW_H - (it.y + it.h / 2), 0);
    // 首次出现时建模型（放最后，便于重复进入该分支也会触发 clear）
    if (grp.children.length === 0) {
      if (it.type === "flower") buildFlower(grp);
      else if (it.type === "oneup") buildOneUp(grp);
      else buildMushroom(grp);
    }
  }
}

// —— 火球与粒子（池化小球/小方块） ——
const fireMeshes: THREE.Mesh[] = [];
function syncFireballs() {
  const w = worldGroup;
  if (!w) return;
  const n = Math.max(fireMeshes.length, game.fireballs.length);
  for (let i = 0; i < n; i++) {
    const f = game.fireballs[i];
    if (!f) {
      if (fireMeshes[i]) fireMeshes[i].visible = false;
      continue;
    }
    let mesh = fireMeshes[i];
    if (!mesh) {
      mesh = new THREE.Mesh(geoFire, mat(0xff7a1a));
      w.add(mesh);
      fireMeshes[i] = mesh;
    }
    mesh.visible = true;
    mesh.position.set(f.x + 3, VIEW_H - (f.y + 3), 0);
  }
}
const partMeshes: THREE.Mesh[] = [];
function syncParticles() {
  const w = worldGroup;
  if (!w) return;
  for (let i = 0; i < 100; i++) {
    const p = game.particles[i];
    if (!p) {
      if (partMeshes[i]) partMeshes[i].visible = false;
      continue;
    }
    let mesh = partMeshes[i];
    if (!mesh) {
      mesh = new THREE.Mesh(boxGeom(6, 6, 6), mat(0xffcc00));
      w.add(mesh);
      partMeshes[i] = mesh;
    }
    mesh.visible = true;
    mesh.position.set(p.x + 5, VIEW_H - (p.y + 2), 0);
  }
}

// —— 相机侧视跟随（略俯视以展现块体顶面，保留横板手感） ——
function updateCamera() {
  const c = camera;
  if (!c) return;
  const cx = game.mario.x + game.mario.w / 2;
  // 横版侧视：摄像机只横向跟随（限制在世界宽度内，水平半视野保留 CAM_HALF_W 余量），
  // 垂直高度与俯视角固定，确保地面始终稳定贴在画面下部，避免竖直跟随让场景“飘空”。
  const tx = Math.max(CAM_HALF_W, Math.min(cx, game.cols * TILE - CAM_HALF_W));
  c.position.x = tx;
  c.position.y = CAM_Y; // 固定高度
  c.lookAt(tx, LOOK_Y, 0);
}

// —— 背景装饰：远景山丘 + 云朵（视差滚动，让天空更丰富） ——
const bgClouds: THREE.Group[] = [];
const bgHills: THREE.Group[] = [];
function wrapRange(val: number, lo: number, hi: number): number {
  const span = hi - lo;
  while (val < lo) val += span;
  while (val > hi) val -= span;
  return val;
}
function initBackground() {
  const s = scene;
  if (!s) return;
  bgClouds.length = 0;
  bgHills.length = 0;

  // 云朵：白色扁平团块，漂浮在天空高处
  const cloudMat = new THREE.MeshToonMaterial({ color: 0xffffff, transparent: true, opacity: 0.92 });
  for (let i = 0; i < 10; i++) {
    const g = new THREE.Group();
    const blob = (ox: number, oy: number, oz: number, scl: number) => {
      const m = new THREE.Mesh(boxGeom(scl, scl * 0.5, scl * 0.34), cloudMat);
      m.position.set(ox, oy, oz);
      g.add(m);
    };
    blob(0, 0, 0, 42);
    blob(34, -4, 8, 26);
    blob(-34, -3, -6, 30);
    g.userData = { seed: i * 173 + 29, par: 0.55 };
    g.position.set(i * 300 - 900, 285 + (i % 4) * 18, -100);
    s.add(g);
    bgClouds.push(g);
  }

  // 远景山丘：低矮钝色丘脊，沉在地平线后
  const hillBody = new THREE.MeshToonMaterial({ color: 0x4f9e5f, transparent: true, opacity: 0.85 });
  const hillTop = new THREE.MeshToonMaterial({ color: 0x6fbf7f, transparent: true, opacity: 0.85 });
  for (let i = 0; i < 11; i++) {
    const g = new THREE.Group();
    const w = 240 + (i % 4) * 70;
    const h = 46 + (i % 5) * 24;
    const body = new THREE.Mesh(boxGeom(w, h, 40), hillBody);
    body.position.set(0, -h / 2, 0);
    g.add(body);
    const top = new THREE.Mesh(boxGeom(w - 20, 10, 42), hillTop);
    top.position.set(0, -4, 0);
    g.add(top);
    g.userData = { seed: i * 131 + 7, par: 0.28 };
    g.position.set(i * 260 - 900, 30, -190);
    s.add(g);
    bgHills.push(g);
  }
}
function updateBackground() {
  const c = camera;
  if (!c) return;
  const cx = c.position.x;
  // 云：水平视差滚动并回绕，保持在视线附近
  for (const g of bgClouds) {
    const x = wrapRange(cx * g.userData.par + g.userData.seed, cx - 1400, cx + 1400);
    g.position.x = x;
  }
  // 山：慢速视差，保持在地面带后方
  for (const g of bgHills) {
    const x = wrapRange(cx * g.userData.par + g.userData.seed, cx - 1500, cx + 1500);
    g.position.x = x;
  }
}

// —— 同步 UI 到 DOM（HUD / 标题 / 结算覆盖层用 Vue 模板渲染） ——
const ui = reactive({ state: "title", score: 0, coins: 0, lives: 0, time: 0, banner: "", bannerT: 0, won: false, flagBonus: 0, finalTime: 0 });
function updateUI() {
  ui.state = game.state;
  ui.score = game.score;
  ui.coins = game.coins;
  ui.lives = game.lives;
  ui.time = game.time;
  ui.banner = game.banner;
  ui.bannerT = game.bannerT;
  ui.won = game.won;
  ui.flagBonus = game.flagBonus;
  ui.finalTime = game.finalTime;
}

// —— 总渲染入口：每次 RAF 同步 3D 并出图 ——
function render() {
  if (!renderer || !scene) return;
  setBackground(game.theme);
  scene.background = bgColor;
  updateCamera(); // 先更新相机，确保瓦片/背景都按当前可见中心定位
  syncTiles();
  syncMario();
  syncEnemies();
  syncItems();
  syncFireballs();
  syncParticles();
  updateBackground();
  updateUI();
  renderer.render(scene, camera!);
}

// ============ 三维场景初始化 ============
// 相机固定高度与俯视角（单位=像素世界）。
// 高度选值使地面(y0~16)落在画面下部：CAM_Y 略高、LOOK_Y 略低于 CAM_Y，形成轻微俯视。
const CAM_Z = 560;
const CAM_Y = 250;
const LOOK_Y = 150;
// 16:9 宽屏水平半视野余量：确保马里奥不被甩出画面边缘
const CAM_HALF_W = 560;
let resizeOb: ResizeObserver | null = null;

function initThree() {
  const host = renderHost.value;
  if (!host) return;
  renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.domElement.style.width = "100%";
  renderer.domElement.style.height = "100%";
  host.appendChild(renderer.domElement);

  scene = new THREE.Scene();
  scene.background = bgColor;
  scene.fog = new THREE.Fog(0x8fd0ff, 700, 2000);
  // 明亮光照：环境光保证所有面可见（不再灰暗死黑）+ 半球光冷暖层次 + 主/补两个方向光
  scene.add(new THREE.AmbientLight(0xffffff, 0.85));
  scene.add(new THREE.HemisphereLight(0xffffff, 0x7788cc, 1.1));
  const sun = new THREE.DirectionalLight(0xffffff, 1.6);
  sun.position.set(260, 460, 520);
  scene.add(sun);
  const fill = new THREE.DirectionalLight(0xcfe4ff, 0.6); // 补光：提亮暗面，增强立体感但不脏
  fill.position.set(-320, 120, -420);
  scene.add(fill);

  camera = new THREE.PerspectiveCamera(50, 1.5, 1, 3000);
  camera.position.set(VIEW_W / 2, CAM_Y, CAM_Z);
  camera.lookAt(VIEW_W / 2, LOOK_Y, 0);

  worldGroup = new THREE.Group();
  scene.add(worldGroup);
  marioModel.visible = false;
  worldGroup.add(marioModel);
  initBackground(); // 云朵 + 远山

  // 随容器尺寸自适应（原生分辨率渲染，避免 CSS 放大导致的模糊）
  resize();
  resizeOb = new ResizeObserver(resize);
  resizeOb.observe(host);
}

/** 按宿主实际像素尺寸设置渲染器与相机宽高比 */
function resize() {
  const host = renderHost.value;
  if (!host || !renderer || !camera) return;
  const w = host.clientWidth;
  const h = host.clientHeight;
  if (w === 0 || h === 0) return;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}

// ============ 渲染主循环 ============
// 固定 60fps 逻辑步进：用时间累加器按 1/60 秒驱动 update()，
// 避免高刷新率（如 144Hz 显示器）下 requestAnimationFrame 频率翻倍导致游戏加速。
let acc = 0;
const FIXED_STEP = 1000 / 60;
function loop(ts: number) {
  if (lastTime === 0) lastTime = ts;
  acc += Math.min(ts - lastTime, 250);
  lastTime = ts;

  let steps = 0;
  while (acc >= FIXED_STEP && steps < 6) {
    update();
    acc -= FIXED_STEP;
    steps++;
  }
  if (acc >= FIXED_STEP) acc = 0; // 极端卡顿后丢弃积压，避免“死亡螺旋”

  render();
  rafId = requestAnimationFrame(loop);
}

// ============ 生命周期 ============
onMounted(() => {
  initThree();
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
  if (resizeOb) {
    resizeOb.disconnect();
    resizeOb = null;
  }
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("keyup", onKeyUp);
  if (renderer) {
    renderer.dispose();
    if (renderer.domElement.parentNode) renderer.domElement.parentNode.removeChild(renderer.domElement);
    renderer = null;
  }
});
</script>

<template>
  <div class="screen-root">
    <div ref="renderHost" class="render-host">
      <!-- HUD -->
      <div v-if="ui.state !== 'title'" class="hud">
        <span>SCORE {{ String(ui.score).padStart(6, "0") }}</span>
        <span>COINS x{{ ui.coins }}</span>
        <span>LIVES {{ ui.lives }} &nbsp; ⏱ {{ ui.time }}</span>
      </div>
      <!-- 关卡切换横幅 -->
      <div v-if="ui.bannerT > 0 && ui.banner" class="banner">{{ ui.banner }}</div>
      <!-- 标题 -->
      <div v-if="ui.state === 'title'" class="overlay">
        <h1>SUPER MARIO</h1>
        <p class="sub">3D 体素复刻 · 横板跳跃</p>
        <p class="keys">←→ 移动 · Space/↑ 跳跃 · X/C 火球</p>
        <p class="tip-line">顶 ?块出金币/蘑菇/火花花 · 顶 B 砖可碎 · 踩敌人消灭</p>
        <p class="tip-line">踩乌龟可缩壳/滑行 · 触旗杆按高度分段得分</p>
        <p class="tip-line">隐藏砖可登高 · 问号块藏着 1UP 绿蘑菇加命</p>
        <p class="start">按 ENTER 开始</p>
      </div>
      <!-- 结算 -->
      <div v-else-if="ui.state === 'clear'" class="overlay">
        <h1 :class="ui.won ? 'win' : 'lose'">{{ ui.won ? "LEVEL CLEAR!" : "GAME OVER" }}</h1>
        <p v-if="ui.won">得分 {{ ui.score }}</p>
        <p v-if="ui.won">旗杆高度奖励 += {{ ui.flagBonus }}</p>
        <p v-if="ui.won">时间奖励 +{{ ui.finalTime * 10 }}</p>
        <p v-if="ui.won">金币 ×{{ ui.coins }} · 已提交到高分榜</p>
        <p v-if="!ui.won">得分 {{ ui.score }}</p>
        <p v-if="!ui.won">金币 {{ ui.coins }} · 本局已上榜</p>
        <p class="start">按 ENTER 再来一局</p>
      </div>
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
  overflow: hidden;
  background: radial-gradient(circle at center, #1a2440 0%, #0a0e1a 80%);
}
.render-host {
  position: relative;
  width: 100%;
  height: 100%; /* 铺满整个可用区域，无黑边；相机随容器宽高比自适应 */
  border: 3px solid #ffcc00;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 0 40px rgba(255, 204, 0, 0.25);
}
.render-host canvas {
  width: 100% !important;
  height: 100% !important;
  display: block;
}
.hud {
  position: absolute;
  top: 3px;
  left: 3px;
  right: 3px;
  height: 26px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 10px;
  font: bold 13px monospace;
  color: #fff;
  background: rgba(0, 0, 0, 0.45);
  border-radius: 6px 6px 0 0;
  text-shadow: 1px 1px 2px #000;
}
.banner {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: #ffcc00;
  font: bold 26px monospace;
  background: rgba(0, 0, 0, 0.55);
  padding: 10px 24px;
  border-radius: 6px;
  white-space: nowrap;
}
.overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  text-align: center;
  font-size: 14px;
}
.overlay h1 {
  color: #ffcc00;
  font-size: 30px;
  margin: 0 0 6px;
}
.overlay h1.win {
  color: #7aff7a;
}
.overlay h1.lose {
  color: #ff5a5a;
}
.overlay .sub {
  color: #fff;
}
.overlay .keys {
  color: #e23b2e;
}
.overlay .tip-line {
  color: #ddd;
}
.overlay .start {
  color: #7aff7a;
  font-size: 18px;
  font-weight: bold;
  margin-top: 8px;
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