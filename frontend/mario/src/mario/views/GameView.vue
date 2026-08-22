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
import { onBeforeUnmount, onMounted, ref } from "vue";
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
// ' ' 空、'X' 实心地/砖、'B' 可顶碎砖、'Q' 问号块（顶出金币）、'#' 已使用块
// 'P' 水管壁、'o' 悬浮金币、'K' 终点城堡
type TileChar = " " | "X" | "B" | "Q" | "#" | "P" | "o" | "K";

const canvasRef = ref<HTMLCanvasElement | null>(null);
// 类型上视为已初始化（onMounted 统一赋值），运行时各绘制入口均以 `if (!ctx) return;` 兜底
let ctx!: CanvasRenderingContext2D;

const authStore = useAuthStore();

// ============ 游戏状态（非响应式，性能优先） ============
const game = {
  cols: 120,
  grid: [] as TileChar[][], // grid[row][col]
  mario: { x: 0, y: 0, w: 14, h: 20, vx: 0, vy: 0, onGround: false, facing: 1, anim: 0 },
  enemies: [] as Enemy[],
  particles: [] as Particle[],
  state: "title" as "title" | "play" | "dead" | "clear",
  score: 0,
  coins: 0,
  lives: 3,
  time: 200,
  level: 1,
  camera: 0,
  frame: 0,
  timer: 0,
  respawnIdle: 0,
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
  alive: boolean;
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
  return t === "X" || t === "B" || t === "Q" || t === "#" || t === "P" || t === "K";
}

// ============ 关卡生成 ============
function buildLevel() {
  const cols = game.cols;
  // 初始化空网格
  const grid: TileChar[][] = [];
  for (let r = 0; r < ROWS; r++) {
    grid.push(new Array<TileChar>(cols).fill(" "));
  }

  // 地面（最底行），保留少量坑洞增加挑战
  for (let c = 0; c < cols; c++) {
    const pit = c === 38 || c === 78 || c === 108;
    grid[ROWS - 1][c] = pit ? " " : "X";
  }

  // 辅助：放置一行块
  const putRow = (row: number, from: number, to: number, t: TileChar) => {
    for (let c = from; c <= to; c++) {
      if (c >= 0 && c < cols) grid[row][c] = t;
    }
  };

  // 问号金币块与砖块层（第 14 行即地面上 5 格，第 15 行即 4 格可顶到）
  const groups: Array<[number, number, TileChar]> = [
    // [起始列, 数量, 类型]
    [6, 4, "B"],
    [12, 1, "Q"],
    [13, 1, "Q"],
    [16, 3, "B"],
    [21, 1, "Q"],
    [24, 2, "B"],
    [27, 1, "Q"],
    [30, 4, "B"],
    [33, 1, "Q"],
    [34, 1, "Q"],
    [42, 3, "B"],
    [46, 1, "Q"],
    [50, 5, "B"],
    [56, 1, "Q"],
    [60, 3, "B"],
    [64, 1, "Q"],
    [70, 4, "B"],
    [75, 1, "Q"],
    [82, 3, "B"],
    [86, 1, "Q"],
    [90, 4, "B"],
    [95, 1, "Q"],
    [100, 3, "B"],
    [104, 1, "Q"],
  ];
  for (const [from, count, t] of groups) {
    putRow(14, from, from + count - 1, t);
  }

  // 部分悬空平台
  putRow(9, 44, 47, "B");
  putRow(9, 52, 55, "B");
  putRow(7, 98, 101, "B");

  // 悬浮金币
  const coinCells: Array<[number, number]> = [
    [14, 12], [15, 12], [22, 11], [25, 11], [28, 11],
    [43, 11], [47, 11], [57, 11], [61, 11], [71, 11],
    [83, 11], [91, 11], [101, 11],
  ];
  for (const [c, r] of coinCells) grid[r][c] = "o";

  // 水管（两座）
  const pipes: Array<[number, number]> = [
    // [列, 高出地面格数]
    [19, 3],
    [66, 4],
    [92, 2],
  ];
  for (const [c, h] of pipes) {
    for (let r = ROWS - 1 - h; r < ROWS - 1; r++) grid[r][c] = "P";
  }

  // 终点城堡 + 旗杆（旗杆用城堡墙带）
  game.flagCol = 115;
  for (let r = ROWS - 6; r < ROWS; r++) grid[r][124] = "K";
  grid[ROWS - 7][124] = "P";
  grid[ROWS - 6][124] = "P";

  game.grid = grid;

  // 敌人（栗子怪）
  game.enemies = [
    { x: 14 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.6, alive: true },
    { x: 32 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.7, alive: true },
    { x: 48 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.5, alive: true },
    { x: 63 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.8, alive: true },
    { x: 84 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.6, alive: true },
    { x: 96 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.9, alive: true },
    { x: 106 * TILE, y: groundTop() - 14, w: 14, h: 14, vx: -0.7, alive: true },
  ];
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
  spawnMario();
}

function spawnMario() {
  const m = game.mario;
  m.x = 2 * TILE;
  m.y = groundTop() - m.h;
  m.vx = 0;
  m.vy = 0;
  m.onGround = true;
  game.camera = 0;
}

/** 通关：结算并提交成绩 */
function doWin() {
  game.state = "clear";
  game.won = true;
  game.finalTime = game.time;
  const timeBonus = game.time * 10;
  game.score += timeBonus;
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
    m.vy = -8;
    m.onGround = false;
    game.timer = 12; // 跳跃缓冲
  }
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
  if (t === "Q") {
    // 问号块：出金币，变已使用
    game.grid[row][col] = "#";
    game.coins += 1;
    game.score += 150;
    spawnCoinBurst(col * TILE + 8, row * TILE);
  } else if (t === "B") {
    // 砖块：顶碎
    game.grid[row][col] = " ";
    game.score += 60;
    spawnBrickBits(col * TILE, row * TILE);
  }
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

    // 敌人
    for (const e of game.enemies) {
      if (!e.alive) continue;
      e.x += e.vx;
      // 碰壁反向
      if (isSolid(tileAt(Math.floor((e.vx > 0 ? e.x + e.w : e.x) / TILE), Math.floor((e.y + e.h) / TILE)))) {
        e.vx *= -1;
      }
      // 与马里奥碰撞
      if (
        m.x < e.x + e.w && m.x + m.w > e.x &&
        m.y < e.y + e.h && m.y + m.h > e.y
      ) {
        const falling = m.vy > 0;
        const above = m.y + m.h - e.y < e.h * 0.6;
        if (falling && above) {
          // 踩扁
          e.alive = false;
          game.score += 200;
          m.vy = -6;
          m.onGround = false;
        } else {
          playerDie();
          return;
        }
      }
    }
    // 移除已被踩灭的敌人
    game.enemies = game.enemies.filter((e) => e.alive);

    // 掉落判定（掉出屏幕或掉进坑）
    if (m.y > VIEW_H) {
      playerDie();
      return;
    }

    // 终点判定
    if (m.x + m.w >= game.flagCol * TILE) {
      doWin();
      return;
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

function drawMario() {
  if (!ctx) return;
  const m = game.mario;
  const px = Math.round(m.x - game.camera);
  const py = Math.round(m.y);
  const flap = Math.sin(game.mario.anim * 0.5);

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
  // 每只脚
  ctx.fillStyle = "#8a4a24";
  if (!m.onGround) {
    // 跳跃叉腿
    ctx.fillRect(px - 1, py + m.h - 3, m.w / 2, 3);
    ctx.fillRect(px + m.w / 2 - 1, py + m.h - 4, m.w / 2, 3);
  } else {
    const step = Math.sin(game.mario.anim * 0.5) > 0 ? 1 : -1;
    ctx.fillRect(px + step, py + m.h - 3, m.w / 2 + 1, 3);
    ctx.fillRect(px + m.w / 2 - step, py + m.h - 3, m.w / 2 + 1, 3);
  }
  void flap;
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
  ctx.fillText("←→ 移动  |  Space/↑ 跳跃", VIEW_W / 2, 180);
  ctx.fillText("顶 ?块出金币 · 顶 B砖可碎 · 踩敌人消灭", VIEW_W / 2, 200);
  ctx.fillText("抵达终点旗杆通关 · R 重新开始", VIEW_W / 2, 220);
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
        `金币 ×${game.coins}`,
        `时间奖励 ${game.finalTime * 10}`,
        `已提交到高分榜`,
      ];
  lines.forEach((ln, i) => ctx.fillText(ln, VIEW_W / 2, 140 + i * 22));

  ctx.font = "bold 16px monospace";
  ctx.fillStyle = "#ffcc00";
  ctx.fillText("按 ENTER 再来一局", VIEW_W / 2, 232);
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
  <div class="mario-page">
    <div class="game-wrap">
      <canvas
        ref="canvasRef"
        :width="VIEW_W"
        :height="VIEW_H"
        class="mario-canvas"
        tabindex="0"
      />
      <div class="tip">当前用户：{{ authStore.user?.username ?? "—" }}</div>
    </div>
  </div>
</template>

<style scoped>
.mario-page {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}
.game-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.mario-canvas {
  width: min(100%, 900px);
  aspect-ratio: 480 / 320;
  background: #000;
  border: 3px solid #ffcc00;
  border-radius: 8px;
  image-rendering: pixelated;
  outline: none;
  box-shadow: 0 0 30px rgba(255, 204, 0, 0.2);
}
.tip {
  font-size: 13px;
  color: #aaa;
}
</style>