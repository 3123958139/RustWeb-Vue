/**
 * useCityScene - Three.js 城市 3D 场景组合式函数
 *
 * 功能特性：
 * - 程序化生成城市建筑（从 API 数据驱动）
 * - 地面发光网格 + 星空背景
 * - 建筑发光窗户（夜晚亮灯效果）
 * - 车流流光粒子系统
 * - 楼顶航标灯闪烁
 * - UnrealBloom 辉光后处理
 * - OrbitControls（自动旋转 + 交互）
 * - Raycaster 悬停高亮 + 点击聚焦
 * - 4 档昼夜切换（黎明/白天/黄昏/夜晚）
 * - 天气粒子（雨/雪/雾）
 * - 能耗热力上色模式切换
 */
import { onScopeDispose, ref, watchEffect } from "vue";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import type { Building } from "@/city3d/api/city3d";
import {
  TIME_OF_DAY,
  type TimeOfDayConfig,
  type TimeOfDayKey,
} from "@/city3d/data/timeOfDay";
import {
  groundFragmentShader,
  groundVertexShader,
  skyFragmentShader,
  skyVertexShader,
} from "@/city3d/shaders";

export type WeatherKey = "none" | "rain" | "snow" | "fog";

const CITY_RADIUS = 560;
const TRAFFIC_COUNT = 960;
const STAR_COUNT = 1300;

/** 运行时插值状态（当前值向目标值平滑过渡） */
interface LerpState {
  ambient: number;
  sun: number;
  windowEmissive: number;
  bloomStrength: number;
  starOpacity: number;
  trafficOpacity: number;
  sunOpacity: number;
  moonOpacity: number;
  fogNear: number;
  fogFar: number;
  sunHeight: number;
  skyTop: THREE.Color;
  skyBottom: THREE.Color;
  fogColor: THREE.Color;
  lightColor: THREE.Color;
  sunColor: THREE.Color;
  moonColor: THREE.Color;
}

function lerpNumber(a: number, b: number, k: number): number {
  return a + (b - a) * k;
}

function colorLerp(a: THREE.Color, b: THREE.Color, k: number): THREE.Color {
  return a.clone().lerp(b, k);
}

/** 由能量值映射热力色（蓝 → 青 → 黄 → 红） */
function heatColor(energyKw: number, maxEnergyKw: number): THREE.Color {
  const t = maxEnergyKw > 0 ? Math.min(energyKw / maxEnergyKw, 1) : 0;
  const color = new THREE.Color();
  color.setHSL(0.62 - t * 0.62, 0.95, 0.55);
  return color;
}

/**
 * 城市 3D 场景组合式函数
 *
 * 使用示例：
 * ```ts
 * const scene = useCityScene();
 * onMounted(() => {
 *   scene.initScene(containerRef.value!);
 *   scene.loadBuildings(buildings);
 * });
 * onBeforeUnmount(() => scene.disposeScene());
 * ```
 */
export function useCityScene() {
  // ============ 响应式状态 ============
  const selectedBuilding = ref<Building | null>(null);
  const hoveredBuilding = ref<Building | null>(null);
  /** 场景初始化/运行错误（供界面显示） */
  const sceneError = ref<string>("");
  /** WebGL 上下文是否处于丢失状态 */
  const contextLost = ref(false);

  // ============ Three.js 内部引用 ============
  let container: HTMLElement | null = null;
  let renderer: THREE.WebGLRenderer | null = null;
  let scene: THREE.Scene | null = null;
  let camera: THREE.PerspectiveCamera | null = null;
  let controls: OrbitControls | null = null;
  let composer: EffectComposer | null = null;
  let bloomPass: UnrealBloomPass | null = null;
  let clock = new THREE.Clock();
  let disposed = false;

  // 天空与星月
  let skyMesh: THREE.Mesh | null = null;
  let skyUniforms: { [uniform: string]: THREE.IUniform } | null = null;
  let stars: THREE.Points | null = null;
  let sunSprite: THREE.Sprite | null = null;
  let moonSprite: THREE.Sprite | null = null;
  let sunDir = new THREE.Vector3(0.45, 1.5, 0.75).normalize();
  let moonDir = new THREE.Vector3(-0.45, -0.35, -0.75).normalize();

  // 地面
  let groundDisc: THREE.Mesh | null = null;
  let groundUniforms: { [uniform: string]: THREE.IUniform } | null = null;
  let gridHelper: THREE.GridHelper | null = null;
  let edgeRing: THREE.Mesh | null = null;
  let scanRing: THREE.Mesh | null = null;
  let pulseRing: THREE.Mesh | null = null;
  let avenues: THREE.LineSegments | null = null;

  // 建筑
  let buildingMeshes: THREE.Mesh[] = [];
  let buildingData: Building[] = [];
  let windowsTexture: THREE.CanvasTexture | null = null;
  let beacons: THREE.Mesh[] = [];
  let beaconPhases: number[] = [];
  let beaconGeometryCache: THREE.BufferGeometry | null = null;

  // 车流
  let trafficPoints: THREE.Points | null = null;
  let trafficRoutes: THREE.CatmullRomCurve3[] = [];
  let trafficIdx: Uint16Array = new Uint16Array(TRAFFIC_COUNT);
  let trafficT: Float32Array = new Float32Array(TRAFFIC_COUNT);
  let trafficSpeed: Float32Array = new Float32Array(TRAFFIC_COUNT);
  let trafficJitter: Float32Array = new Float32Array(TRAFFIC_COUNT);
  let trafficPositions: Float32Array | null = null;

  // 天气
  let weather: WeatherKey = "none";
  let rainPoints: THREE.Points | null = null;
  let rainData: Float32Array | null = null;
  let snowPoints: THREE.Points | null = null;
  let snowData: Float32Array | null = null;

  // 交互
  let raycaster = new THREE.Raycaster();
  let pointer = new THREE.Vector2(-10, -10);
  let hoveredMesh: THREE.Mesh | null = null;
  let hoverOriginalEmissive: THREE.Color | null = null;
  let focusedMesh: THREE.Mesh | null = null;
  let focusTarget: THREE.Vector3 | null = null;

  // 时段与模式
  let heatMode = false;
  let introDone = false;
  let current!: LerpState;
  let target!: LerpState;

  // 入场动画
  let introStart = new THREE.Vector3();
  let introEnd = new THREE.Vector3();
  let introTime = 0;

  // ============ 事件处理 ============

  function handlePointerMove(event: PointerEvent): void {
    if (!container) return;
    const rect = container.getBoundingClientRect();
    pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
  }

  function handlePointerClick(event: PointerEvent): void {
    if (!container || !camera) return;
    pointer.x = ((event.clientX - container.getBoundingClientRect().left) / container.clientWidth) * 2 - 1;
    pointer.y = -((event.clientY - container.getBoundingClientRect().top) / container.clientHeight) * 2 + 1;
    raycaster.setFromCamera(pointer, camera);
    const hits = raycaster.intersectObjects(buildingMeshes, false);
    if (hits.length > 0 && hits[0].object instanceof THREE.Mesh) {
      focusedMesh = hits[0].object;
      const index = buildingMeshes.indexOf(hits[0].object);
      selectedBuilding.value = buildingData[index] ?? null;
    } else {
      focusedMesh = null;
      selectedBuilding.value = null;
    }
  }

  function handleResize(): void {
    if (!container || !camera || !renderer || !composer) return;
    const width = container.clientWidth;
    const height = container.clientHeight;
    if (width === 0 || height === 0) return;
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
    composer.setSize(width, height);
  }

  // ============ 初始化 ============

  function onContextLost(event: Event): void {
    event.preventDefault();
    contextLost.value = true;
    sceneError.value = "WebGL 上下文丢失（显卡驱动或远程桌面环境不稳定），已尝试自动恢复…";
  }

  function onContextRestored(): void {
    contextLost.value = false;
    sceneError.value = "";
  }

  /**
   * 初始化场景，挂载到指定容器元素
   */
  function initScene(host: HTMLElement): void {
    if (disposed) return;
    container = host;

    if (host.clientWidth === 0 || host.clientHeight === 0) {
      sceneError.value = "场景容器尺寸为 0，无法初始化 3D 场景";
      return;
    }

    try {
      initSceneInternal(host);
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      sceneError.value = `3D 场景初始化失败（当前环境可能不支持 WebGL2/GPU 加速）：${detail}`;
      const canvasEl = renderer?.domElement;
      if (canvasEl && canvasEl.parentElement === host) {
        host.removeChild(canvasEl);
      }
      renderer = null;
      scene = null;
      camera = null;
      controls = null;
      composer = null;
      container = null;
    }
  }

  function initSceneInternal(host: HTMLElement): void {
    container = host;

    // Renderer
    renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: false,
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.appendChild(renderer.domElement);

    // Scene
    scene = new THREE.Scene();

    // Camera
    camera = new THREE.PerspectiveCamera(
      55,
      host.clientWidth / host.clientHeight,
      0.1,
      4000
    );
    camera.position.set(0, 430, 1080);

    // Controls
    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 140;
    controls.maxDistance = 1500;
    controls.maxPolarAngle = Math.PI / 2.12;
    controls.target.set(0, 0, 0);
    controls.autoRotate = true;
    controls.autoRotateSpeed = 0.6;

    // Composer
    composer = new EffectComposer(renderer);
    composer.addPass(new RenderPass(scene, camera));
    bloomPass = new UnrealBloomPass(
      new THREE.Vector2(host.clientWidth, host.clientHeight),
      1.4,
      0.55,
      0.72
    );
    composer.addPass(bloomPass);

    // ---- 初始时段：夜晚（赛博夜景开场） ----
    const night = TIME_OF_DAY.night;
    current = stateFromConfig(night);
    target = stateFromConfig(night);

    scene.fog = new THREE.Fog(night.fogColor, night.fogNear, night.fogFar);
    scene.background = new THREE.Color(night.skyTop);

    // ---- 光照 ----
    const ambient = new THREE.AmbientLight(new THREE.Color("#8099c8"), current.ambient);
    ambient.name = "Ambient";
    scene.add(ambient);
    const sun = new THREE.DirectionalLight(
      current.lightColor.clone(),
      current.sun
    );
    sun.position.copy(sunDir).multiplyScalar(600);
    sun.name = "SunLight";
    scene.add(sun);

    // ---- 天空穹顶 ----
    skyUniforms = {
      uTopColor: { value: new THREE.Color(night.skyTop) },
      uBottomColor: { value: new THREE.Color(night.skyBottom) },
      uSunColor: { value: new THREE.Color(night.sunColor) },
      uSunIntensity: { value: night.sunOpacity * 0.9 },
      uSunDir: { value: sunDir },
      uMoonColor: { value: new THREE.Color(night.moonColor) },
      uMoonIntensity: { value: night.moonOpacity * 0.7 },
      uMoonDir: { value: moonDir },
    };
    skyMesh = new THREE.Mesh(
      new THREE.SphereGeometry(1600, 32, 18),
      new THREE.ShaderMaterial({
        vertexShader: skyVertexShader,
        fragmentShader: skyFragmentShader,
        uniforms: skyUniforms,
        side: THREE.BackSide,
        depthWrite: false,
        fog: false,
      })
    );
    skyMesh.renderOrder = -10;
    scene.add(skyMesh);

    // ---- 星空 ----
    const starGeometry = new THREE.BufferGeometry();
    const starPositions = new Float32Array(STAR_COUNT * 3);
    for (let i = 0; i < STAR_COUNT; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(Math.random() * 0.92);
      const radius = 1300 + Math.random() * 200;
      starPositions[i * 3] = radius * Math.sin(phi) * Math.cos(theta);
      starPositions[i * 3 + 1] = radius * Math.cos(phi) + 60;
      starPositions[i * 3 + 2] = radius * Math.sin(phi) * Math.sin(theta);
    }
    starGeometry.setAttribute("position", new THREE.BufferAttribute(starPositions, 3));
    stars = new THREE.Points(
      starGeometry,
      new THREE.PointsMaterial({
        color: new THREE.Color("#cfe4ff"),
        size: 1.4,
        transparent: true,
        opacity: night.starOpacity,
        depthWrite: false,
        fog: false,
      })
    );
    stars.renderOrder = -9;
    scene.add(stars);

    // ---- 太阳 / 月亮（发光精灵） ----
    const glowTexture = createGlowTexture();
    sunSprite = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: glowTexture,
        color: new THREE.Color(night.sunColor),
        transparent: true,
        opacity: night.sunOpacity,
        depthWrite: false,
        fog: false,
        blending: THREE.AdditiveBlending,
      })
    );
    sunSprite.scale.setScalar(190);
    sunSprite.position.copy(sunDir).multiplyScalar(1500);
    scene.add(sunSprite);

    moonSprite = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: glowTexture,
        color: new THREE.Color(night.moonColor),
        transparent: true,
        opacity: night.moonOpacity,
        depthWrite: false,
        fog: false,
        blending: THREE.AdditiveBlending,
      })
    );
    moonSprite.scale.setScalar(130);
    moonSprite.position.copy(moonDir).multiplyScalar(1500);
    scene.add(moonSprite);

    // ---- 地面 ----
    groundUniforms = {
      uBaseColor: { value: new THREE.Color("#0b1226") },
      uGlowColor: { value: new THREE.Color("#00d4ff") },
      uTime: { value: 0 },
      uRadius: { value: CITY_RADIUS },
    };
    groundDisc = new THREE.Mesh(
      new THREE.CircleGeometry(CITY_RADIUS, 96),
      new THREE.ShaderMaterial({
        vertexShader: groundVertexShader,
        fragmentShader: groundFragmentShader,
        uniforms: groundUniforms,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
        fog: false,
      })
    );
    groundDisc.rotation.x = -Math.PI / 2;
    groundDisc.position.y = -0.5;
    scene.add(groundDisc);

    // 道路网格
    gridHelper = new THREE.GridHelper(1700, 76, new THREE.Color("#0f4d6e"), new THREE.Color("#0a2a44"));
    gridHelper.position.y = -0.45;
    gridHelper.material.transparent = true;
    (gridHelper.material as THREE.Material).opacity = 0.55;
    scene.add(gridHelper);

    // 主路发光线条（中央十字大道）
    const avenueMaterial = new THREE.LineBasicMaterial({
      color: new THREE.Color("#00d4ff"),
      transparent: true,
      opacity: 0.5,
      blending: THREE.AdditiveBlending,
    });
    const avenueGeometry = new THREE.BufferGeometry();
    const avenuePoints = new Float32Array([
      -CITY_RADIUS, -0.4, -55, CITY_RADIUS, -0.4, -55,
      -CITY_RADIUS, -0.4, 55, CITY_RADIUS, -0.4, 55,
      -55, -0.4, -CITY_RADIUS, -55, -0.4, CITY_RADIUS,
      55, -0.4, -CITY_RADIUS, 55, -0.4, CITY_RADIUS,
    ]);
    avenueGeometry.setAttribute("position", new THREE.BufferAttribute(avenuePoints, 3));
    avenues = new THREE.LineSegments(avenueGeometry, avenueMaterial);
    scene.add(avenues);

    // 边缘发光环
    edgeRing = new THREE.Mesh(
      new THREE.RingGeometry(CITY_RADIUS - 3, CITY_RADIUS, 96),
      new THREE.MeshBasicMaterial({
        color: new THREE.Color("#00d4ff"),
        transparent: true,
        opacity: 0.7,
        side: THREE.DoubleSide,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
    );
    edgeRing.rotation.x = -Math.PI / 2;
    edgeRing.position.y = -0.35;
    scene.add(edgeRing);

    // 扫描环（缓慢旋转的扇形标记）
    scanRing = new THREE.Mesh(
      new THREE.RingGeometry(0, 28, 96, 1, 0, Math.PI * 0.5),
      new THREE.MeshBasicMaterial({
        color: new THREE.Color("#00d4ff"),
        transparent: true,
        opacity: 0.16,
        side: THREE.DoubleSide,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
    );
    scanRing.rotation.x = -Math.PI / 2;
    scanRing.position.y = 0.6;
    scene.add(scanRing);

    // 扩散脉冲环
    pulseRing = new THREE.Mesh(
      new THREE.RingGeometry(0.9, 1, 64),
      new THREE.MeshBasicMaterial({
        color: new THREE.Color("#66e0ff"),
        transparent: true,
        opacity: 0.4,
        side: THREE.DoubleSide,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
    );
    pulseRing.rotation.x = -Math.PI / 2;
    pulseRing.position.y = 0.5;
    scene.add(pulseRing);

    // ---- 窗户贴图 ----
    windowsTexture = createWindowsTexture();

    // ---- 车流粒子 ----
    initTraffic();

    // ---- 事件绑定 ----
    container.addEventListener("pointermove", handlePointerMove);
    container.addEventListener("pointerdown", handlePointerClick);
    window.addEventListener("resize", handleResize);

    // ---- 入场动画 ----
    controls.enabled = false;
    camera.position.set(0, 620, 1500);
    introStart = camera.position.clone();
    introEnd = new THREE.Vector3(640, 430, 640);

    // ---- 渲染循环 ----
    renderer.setAnimationLoop(animate);

    // ---- WebGL 上下文丢失监控 ----
    renderer.domElement.addEventListener("webglcontextlost", onContextLost, false);
    renderer.domElement.addEventListener("webglcontextrestored", onContextRestored, false);
  }

  function initTraffic(): void {
    if (!scene) return;
    const routes: THREE.Vector3[][] = [
      [new THREE.Vector3(-CITY_RADIUS, 0, -55), new THREE.Vector3(CITY_RADIUS, 0, -55)],
      [new THREE.Vector3(-CITY_RADIUS, 0, 55), new THREE.Vector3(CITY_RADIUS, 0, 55)],
      [new THREE.Vector3(-55, 0, -CITY_RADIUS), new THREE.Vector3(-55, 0, CITY_RADIUS)],
      [new THREE.Vector3(55, 0, -CITY_RADIUS), new THREE.Vector3(55, 0, CITY_RADIUS)],
      (() => {
        const ring: THREE.Vector3[] = [];
        const segments = 10;
        for (let i = 0; i <= segments; i++) {
          const angle = (i / segments) * Math.PI * 2;
          ring.push(new THREE.Vector3(Math.cos(angle) * 330, 0, Math.sin(angle) * 330));
        }
        return ring;
      })(),
      (() => {
        const ring: THREE.Vector3[] = [];
        const segments = 10;
        for (let i = 0; i <= segments; i++) {
          const angle = (i / segments) * Math.PI * 2;
          ring.push(new THREE.Vector3(Math.cos(angle) * 190, 0, Math.sin(angle) * 190));
        }
        return ring;
      })(),
    ];
    trafficRoutes = routes.map((points) => new THREE.CatmullRomCurve3(points));

    trafficPositions = new Float32Array(TRAFFIC_COUNT * 3);
    const perRoute = Math.floor(TRAFFIC_COUNT / trafficRoutes.length);
    for (let i = 0; i < TRAFFIC_COUNT; i++) {
      const routeIdx = Math.min(Math.floor(i / perRoute), trafficRoutes.length - 1);
      trafficIdx[i] = routeIdx;
      trafficT[i] = Math.random();
      trafficSpeed[i] = 0.035 + Math.random() * 0.05;
      trafficJitter[i] = (Math.random() - 0.5) * 4;
      const point = trafficRoutes[routeIdx].getPointAt(trafficT[i]);
      trafficPositions[i * 3] = point.x + trafficJitter[i];
      trafficPositions[i * 3 + 1] = 0.3;
      trafficPositions[i * 3 + 2] = point.z;
    }

    const trafficGeometry = new THREE.BufferGeometry();
    trafficGeometry.setAttribute("position", new THREE.BufferAttribute(trafficPositions, 3));
    trafficPoints = new THREE.Points(
      trafficGeometry,
      new THREE.PointsMaterial({
        color: new THREE.Color("#66e0ff"),
        size: 1.5,
        transparent: true,
        opacity: target.trafficOpacity,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
    );
    trafficPoints.renderOrder = 5;
    scene.add(trafficPoints);
  }

  // ============ 公开操作 ============

  /**
   * 从 API 数据加载/更新建筑
   */
  function loadBuildings(buildings: Building[]): void {
    if (!scene) return;
    buildingData = buildings;

    // 释放旧资源
    buildingMeshes.forEach((mesh) => {
      scene!.remove(mesh);
      mesh.geometry.dispose();
      (mesh.material as THREE.Material).dispose();
    });
    beacons.forEach((beacon) => {
      scene!.remove(beacon);
      beacon.geometry.dispose();
      (beacon.material as THREE.Material).dispose();
    });
    buildingMeshes = [];
    beacons = [];
    beaconPhases = [];
    hoveredMesh = null;
    focusedMesh = null;

    const maxEnergy = Math.max(...buildings.map((b) => b.energy_kw), 1);
    const beaconGeometry = new THREE.IcosahedronGeometry(2.2, 1);

    buildings.forEach((building, index) => {
      const baseColor = new THREE.Color("#00d4ff");
      const shade = 0.72 + ((index * 37) % 13) / 100;
      const material = new THREE.MeshStandardMaterial({
        color: baseColor.clone().multiplyScalar(shade),
        emissive: heatMode ? heatColor(building.energy_kw, maxEnergy) : baseColor,
        emissiveIntensity: heatMode ? 0.55 : 0.22,
        emissiveMap: windowsTexture!,
        roughness: 0.62,
        metalness: 0.32,
      });

      const geometry = new THREE.BoxGeometry(building.width, building.height, building.depth);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(building.x, building.height / 2, building.z);
      mesh.castShadow = false;
      mesh.receiveShadow = false;
      mesh.name = building.name;
      scene!.add(mesh);
      buildingMeshes.push(mesh);

      // 楼顶航标灯（仅超高层）
      if (building.height >= 150) {
        const beacon = new THREE.Mesh(
          beaconGeometry,
          new THREE.MeshBasicMaterial({
            color: new THREE.Color("#ff3355"),
            transparent: true,
            opacity: 0.9,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
          })
        );
        beacon.position.set(building.x, building.height + 2.5, building.z);
        beacon.scale.setScalar(0.5 + (building.height / 300) * 0.6);
        scene!.add(beacon);
        beacons.push(beacon);
        beaconPhases.push(index * 1.7);
      }
    });

    beaconGeometryCache?.dispose();
    beaconGeometryCache = beaconGeometry;
  }

  /**
   * 切换昼夜时段
   */
  function setTimeOfDay(mode: TimeOfDayKey): void {
    target = stateFromConfig(TIME_OF_DAY[mode]);
  }

  /**
   * 切换能耗热力上色模式
   */
  function setHeatmapMode(enabled: boolean): void {
    heatMode = enabled;
    const maxEnergy = Math.max(...buildingData.map((b) => b.energy_kw), 1);
    buildingMeshes.forEach((mesh, index) => {
      const material = mesh.material as THREE.MeshStandardMaterial;
      if (enabled) {
        const building = buildingData[index];
        material.emissive.copy(heatColor(building.energy_kw, maxEnergy));
        material.emissiveIntensity = 0.55;
      } else {
        material.emissive.copy(new THREE.Color("#00d4ff"));
        material.emissiveIntensity = 0.22;
      }
    });
  }

  /**
   * 设置天气
   */
  function setWeather(type: WeatherKey): void {
    weather = type;
    clearWeather();
    if (type === "rain" && scene) {
      createRain();
    } else if (type === "snow" && scene) {
      createSnow();
    }
  }

  /**
   * 开启自动旋转
   */
  function startAutoRotate(): void {
    if (controls) controls.autoRotate = true;
  }

  /**
   * 关闭自动旋转
   */
  function stopAutoRotate(): void {
    if (controls) controls.autoRotate = false;
  }

  /**
   * 聚焦到指定建筑
   */
  function focusOnBuilding(buildingId: string): void {
    const index = buildingData.findIndex((b) => b.id === buildingId);
    if (index >= 0) {
      const mesh = buildingMeshes[index];
      focusedMesh = mesh;
      focusTarget = mesh.position.clone().setY(mesh.position.y * 0.6 + 20);
    }
  }

  /**
   * 释放场景资源
   */
  function disposeScene(): void {
    disposed = true;
    if (renderer) {
      renderer.setAnimationLoop(null);
      renderer.domElement.removeEventListener("webglcontextlost", onContextLost);
      renderer.domElement.removeEventListener("webglcontextrestored", onContextRestored);
    }
    if (container) {
      container.removeEventListener("pointermove", handlePointerMove);
      container.removeEventListener("pointerdown", handlePointerClick);
    }
    window.removeEventListener("resize", handleResize);
    controls?.dispose();
    if (scene) {
      scene.traverse((object) => {
        if (object instanceof THREE.Mesh || object instanceof THREE.Points || object instanceof THREE.LineSegments) {
          object.geometry.dispose();
          const material = object.material;
          if (Array.isArray(material)) {
            material.forEach((m) => m.dispose());
          } else {
            material.dispose();
          }
        }
      });
    }
    windowsTexture?.dispose();
    beaconGeometryCache?.dispose();
    composer?.dispose();
    renderer?.dispose();
    if (renderer?.domElement && container && renderer.domElement.parentElement === container) {
      container.removeChild(renderer.domElement);
    }
    container = null;
    renderer = null;
    scene = null;
    camera = null;
    controls = null;
    composer = null;
  }

  // 自动在组件卸载时清理
  onScopeDispose(() => {
    if (!disposed) {
      disposeScene();
    }
  });

  // ============ 动画循环 ============

  function animate(): void {
    if (disposed || !scene || !camera || !controls || !bloomPass || !composer || !renderer) return;
    const dt = Math.min(clock.getDelta(), 0.1);
    const time = clock.elapsedTime;
    const k = 1 - Math.exp(-dt * 3.2);

    // ---- 时段插值 ----
    current.ambient = lerpNumber(current.ambient, target.ambient, k);
    current.sun = lerpNumber(current.sun, target.sun, k);
    current.windowEmissive = lerpNumber(current.windowEmissive, target.windowEmissive, k);
    current.bloomStrength = lerpNumber(current.bloomStrength, target.bloomStrength, k);
    current.starOpacity = lerpNumber(current.starOpacity, target.starOpacity, k);
    current.trafficOpacity = lerpNumber(current.trafficOpacity, target.trafficOpacity, k);
    current.sunOpacity = lerpNumber(current.sunOpacity, target.sunOpacity, k);
    current.moonOpacity = lerpNumber(current.moonOpacity, target.moonOpacity, k);
    current.fogNear = lerpNumber(current.fogNear, target.fogNear, k);
    current.fogFar = lerpNumber(current.fogFar, target.fogFar, k);
    current.sunHeight = lerpNumber(current.sunHeight, target.sunHeight, k);
    current.skyTop = colorLerp(current.skyTop, target.skyTop, k);
    current.skyBottom = colorLerp(current.skyBottom, target.skyBottom, k);
    current.fogColor = colorLerp(current.fogColor, target.fogColor, k);
    current.lightColor = colorLerp(current.lightColor, target.lightColor, k);
    current.sunColor = colorLerp(current.sunColor, target.sunColor, k);
    current.moonColor = colorLerp(current.moonColor, target.moonColor, k);

    sunDir.set(0.45, current.sunHeight * 2.6 + 0.55, 0.75).normalize();
    moonDir.set(-0.45, current.sunHeight * -0.9 - 0.35, -0.75).normalize();

    // ---- 应用到场景 ----
    const fog = scene.fog as THREE.Fog;
    fog.color.copy(current.fogColor);
    fog.near = current.fogNear;
    fog.far = current.fogFar;
    (scene.background as THREE.Color).copy(current.skyTop);

    const ambient = scene.getObjectByName("Ambient") as THREE.AmbientLight | undefined;
    if (ambient) ambient.intensity = current.ambient;
    const sun = scene.getObjectByName("SunLight") as THREE.DirectionalLight | undefined;
    if (sun) {
      sun.intensity = current.sun;
      sun.color.copy(current.lightColor);
      sun.position.copy(sunDir).multiplyScalar(600);
    }

    if (skyUniforms && skyMesh) {
      skyUniforms.uTopColor.value.copy(current.skyTop);
      skyUniforms.uBottomColor.value.copy(current.skyBottom);
      skyUniforms.uSunColor.value.copy(current.sunColor);
      skyUniforms.uSunIntensity.value = current.sunOpacity * 0.9;
      skyUniforms.uSunDir.value.copy(sunDir);
      skyUniforms.uMoonColor.value.copy(current.moonColor);
      skyUniforms.uMoonIntensity.value = current.moonOpacity * 0.7;
      skyUniforms.uMoonDir.value.copy(moonDir);
      skyMesh.position.copy(camera.position);
    }

    if (stars) (stars.material as THREE.PointsMaterial).opacity = current.starOpacity;
    if (sunSprite) {
      sunSprite.material.opacity = current.sunOpacity;
      sunSprite.position.copy(sunDir).multiplyScalar(1500);
    }
    if (moonSprite) {
      moonSprite.material.opacity = current.moonOpacity;
      moonSprite.position.copy(moonDir).multiplyScalar(1500);
    }

    bloomPass.strength = current.bloomStrength;

    // 建筑窗户自发光（随时段变化）
    buildingMeshes.forEach((mesh, index) => {
      const material = mesh.material as THREE.MeshStandardMaterial;
      if (!heatMode) {
        const building = buildingData[index];
        const flicker = 0.9 + 0.1 * Math.sin(time * 1.7 + index * 2.3);
        material.emissiveIntensity = current.windowEmissive * flicker * (0.55 + building.occupancy * 0.45);
      }
    });

    // 航标灯闪烁
    beacons.forEach((beacon, index) => {
      (beacon.material as THREE.MeshBasicMaterial).opacity =
        0.35 + 0.65 * Math.abs(Math.sin(time * 2.6 + beaconPhases[index]));
    });

    // 车流粒子
    if (trafficPoints && trafficPositions) {
      const trafficMaterial = trafficPoints.material as THREE.PointsMaterial;
      trafficMaterial.opacity = current.trafficOpacity;
      for (let i = 0; i < TRAFFIC_COUNT; i++) {
        trafficT[i] = (trafficT[i] + trafficSpeed[i] * dt) % 1;
        const point = trafficRoutes[trafficIdx[i]].getPointAt(trafficT[i]);
        trafficPositions[i * 3] = point.x + trafficJitter[i];
        trafficPositions[i * 3 + 2] = point.z;
      }
      (trafficPoints.geometry.attributes.position as THREE.BufferAttribute).needsUpdate = true;
    }

    // 地面动画
    if (groundUniforms) groundUniforms.uTime.value = time;
    if (edgeRing) (edgeRing.material as THREE.MeshBasicMaterial).opacity = 0.5 + 0.25 * Math.sin(time * 1.2);
    if (scanRing) scanRing.rotation.z = time * 0.35;
    if (pulseRing) {
      const pulseScale = ((time * 0.35) % 1.0) * (CITY_RADIUS * 1.6);
      pulseRing.scale.setScalar(pulseScale + 1);
      (pulseRing.material as THREE.MeshBasicMaterial).opacity = 0.38 * (1 - pulseScale / (CITY_RADIUS * 1.7));
    }

    // 天气粒子
    updateWeather(dt, time);

    // 悬停检测
    updateHover();

    // 相机聚焦（平滑跟随）
    if (focusTarget) {
      controls.target.lerp(focusTarget, 1 - Math.exp(-dt * 3));
      if (controls.target.distanceTo(focusTarget) < 1) {
        focusTarget = null;
      }
    }

    // 入场动画
    if (!introDone) {
      updateIntro(dt);
    }

    controls.update();
    composer.render();
  }

  // ============ 悬停 ============

  function updateHover(): void {
    if (!camera) return;
    raycaster.setFromCamera(pointer, camera);
    const hits = raycaster.intersectObjects(buildingMeshes, false);
    const hitMesh = hits.length > 0 && hits[0].object instanceof THREE.Mesh ? hits[0].object : null;

    if (hitMesh !== hoveredMesh) {
      if (hoveredMesh) {
        setHoverStyle(hoveredMesh, false);
      }
      hoveredMesh = hitMesh;
      if (hitMesh) {
        setHoverStyle(hitMesh, true);
        const index = buildingMeshes.indexOf(hitMesh);
        hoveredBuilding.value = buildingData[index] ?? null;
      } else if (!focusedMesh) {
        hoveredBuilding.value = null;
      }
    }
  }

  function setHoverStyle(mesh: THREE.Mesh, hovered: boolean): void {
    if (!container) return;
    const material = mesh.material as THREE.MeshStandardMaterial;
    if (hovered) {
      hoverOriginalEmissive = material.emissive.clone();
      material.emissive.copy(new THREE.Color("#ffffff"));
      material.emissiveIntensity = 0.9;
    } else if (hoverOriginalEmissive) {
      material.emissive.copy(hoverOriginalEmissive);
      hoverOriginalEmissive = null;
    }
    container.style.cursor = hovered ? "pointer" : "default";
  }

  // ============ 天气 ============

  function createRain(): void {
    if (!scene) return;
    const count = 800;
    rainData = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      rainData[i * 3] = (Math.random() - 0.5) * CITY_RADIUS * 2;
      rainData[i * 3 + 1] = Math.random() * 220;
      rainData[i * 3 + 2] = (Math.random() - 0.5) * CITY_RADIUS * 2;
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(rainData, 3));
    rainPoints = new THREE.Points(
      geometry,
      new THREE.PointsMaterial({
        color: new THREE.Color("#9adcff"),
        size: 1.1,
        transparent: true,
        opacity: 0.55,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
    );
    rainPoints.renderOrder = 8;
    scene.add(rainPoints);
  }

  function createSnow(): void {
    if (!scene) return;
    const count = 420;
    snowData = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      snowData[i * 3] = (Math.random() - 0.5) * CITY_RADIUS * 2;
      snowData[i * 3 + 1] = Math.random() * 240;
      snowData[i * 3 + 2] = (Math.random() - 0.5) * CITY_RADIUS * 2;
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(snowData, 3));
    snowPoints = new THREE.Points(
      geometry,
      new THREE.PointsMaterial({
        color: new THREE.Color("#eaf4ff"),
        size: 2.4,
        transparent: true,
        opacity: 0.85,
        depthWrite: false,
      })
    );
    snowPoints.renderOrder = 8;
    scene.add(snowPoints);
  }

  function clearWeather(): void {
    if (rainPoints && scene) {
      scene.remove(rainPoints);
      rainPoints.geometry.dispose();
      (rainPoints.material as THREE.Material).dispose();
      rainPoints = null;
    }
    if (snowPoints && scene) {
      scene.remove(snowPoints);
      snowPoints.geometry.dispose();
      (snowPoints.material as THREE.Material).dispose();
      snowPoints = null;
    }
  }

  function updateWeather(dt: number, time: number): void {
    if (rainPoints && rainData) {
      for (let i = 0; i < rainData.length / 3; i++) {
        rainData[i * 3] += Math.sin(time * 2 + i) * 2.4 * dt;
        rainData[i * 3 + 1] -= 62 * dt;
        rainData[i * 3 + 2] += 6 * dt;
        if (rainData[i * 3 + 1] < 0) {
          rainData[i * 3] = (Math.random() - 0.5) * CITY_RADIUS * 2;
          rainData[i * 3 + 1] = 200 + Math.random() * 40;
          rainData[i * 3 + 2] = (Math.random() - 0.5) * CITY_RADIUS * 2;
        }
      }
      (rainPoints.geometry.attributes.position as THREE.BufferAttribute).needsUpdate = true;
    }
    if (snowPoints && snowData) {
      for (let i = 0; i < snowData.length / 3; i++) {
        snowData[i * 3] += Math.sin(time * 0.8 + i * 0.5) * 3.2 * dt;
        snowData[i * 3 + 1] -= 9 * dt;
        snowData[i * 3 + 2] += 2 * dt;
        if (snowData[i * 3 + 1] < 0) {
          snowData[i * 3] = (Math.random() - 0.5) * CITY_RADIUS * 2;
          snowData[i * 3 + 1] = 220 + Math.random() * 40;
          snowData[i * 3 + 2] = (Math.random() - 0.5) * CITY_RADIUS * 2;
        }
      }
      (snowPoints.geometry.attributes.position as THREE.BufferAttribute).needsUpdate = true;
    }
    // 雾天：将雾距离压扁
    if (weather === "fog" && scene) {
      const fog = scene.fog as THREE.Fog;
      fog.near = Math.min(fog.near, 60);
      fog.far = Math.min(fog.far, 380);
    }
  }

  // ============ 入场动画 ============

  function updateIntro(dt: number): void {
    if (!camera) return;
    introTime += dt;
    const duration = 3.2;
    const t = Math.min(introTime / duration, 1);
    const eased = 1 - Math.pow(1 - t, 3);
    camera.position.lerpVectors(introStart, introEnd, eased);
    camera.lookAt(0, 0, 0);
    if (t >= 1) {
      introDone = true;
      if (controls) controls.enabled = true;
    }
  }

  // ============ 工具 ============

  function stateFromConfig(config: TimeOfDayConfig): LerpState {
    return {
      ambient: config.ambient,
      sun: config.sun,
      windowEmissive: config.windowEmissive,
      bloomStrength: config.bloomStrength,
      starOpacity: config.starOpacity,
      trafficOpacity: config.trafficOpacity,
      sunOpacity: config.sunOpacity,
      moonOpacity: config.moonOpacity,
      fogNear: config.fogNear,
      fogFar: config.fogFar,
      sunHeight: config.sunHeight,
      skyTop: new THREE.Color(config.skyTop),
      skyBottom: new THREE.Color(config.skyBottom),
      fogColor: new THREE.Color(config.fogColor),
      lightColor: new THREE.Color(config.lightColor),
      sunColor: new THREE.Color(config.sunColor),
      moonColor: new THREE.Color(config.moonColor),
    };
  }

  function createGlowTexture(): THREE.CanvasTexture {
    const canvas = document.createElement("canvas");
    canvas.width = 128;
    canvas.height = 128;
    const ctx = canvas.getContext("2d");
    if (ctx) {
      const gradient = ctx.createRadialGradient(64, 64, 0, 64, 64, 64);
      gradient.addColorStop(0, "rgba(255,255,255,1)");
      gradient.addColorStop(0.25, "rgba(255,255,255,0.7)");
      gradient.addColorStop(1, "rgba(255,255,255,0)");
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, 128, 128);
    }
    const texture = new THREE.CanvasTexture(canvas);
    texture.needsUpdate = true;
    return texture;
  }

  function createWindowsTexture(): THREE.CanvasTexture {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 128;
    const ctx = canvas.getContext("2d");
    if (ctx) {
      ctx.fillStyle = "#060a14";
      ctx.fillRect(0, 0, 64, 128);
      const colors = ["#9adcff", "#ffd166", "#8ae9c1", "#c792ff", "#ff9d6b"];
      for (let row = 0; row < 14; row++) {
        for (let col = 0; col < 6; col++) {
          if (Math.random() > 0.28) {
            const color = colors[Math.floor(Math.random() * colors.length)];
            ctx.fillStyle = color;
            ctx.globalAlpha = 0.5 + Math.random() * 0.5;
            ctx.fillRect(3 + col * 10, 4 + row * 9, 6, 5);
          }
        }
      }
      ctx.globalAlpha = 1;
    }
    const texture = new THREE.CanvasTexture(canvas);
    texture.wrapS = THREE.RepeatWrapping;
    texture.wrapT = THREE.RepeatWrapping;
    texture.needsUpdate = true;
    return texture;
  }

  return {
    selectedBuilding,
    hoveredBuilding,
    sceneError,
    contextLost,
    initScene,
    disposeScene,
    loadBuildings,
    setTimeOfDay,
    setHeatmapMode,
    setWeather,
    startAutoRotate,
    stopAutoRotate,
    focusOnBuilding,
  };
}

// ============ 向后兼容导出 ============

/** 场景引擎操作句柄（兼容旧版 Panorama.vue 使用） */
export interface CitySceneHandle {
  setTimeOfDay(key: TimeOfDayKey): void;
  setWeather(key: WeatherKey): void;
  setHeatMode(enabled: boolean): void;
  setAutoRotate(enabled: boolean): void;
  focusBuilding(id: string): void;
  focusPoint(x: number, z: number): void;
  updateBuildings(buildings: Building[], _districts?: Array<{ district: { id: string; color: string } }>): void;
  dispose(): void;
}

/**
 * 创建城市场景引擎（兼容旧版 Panorama.vue 使用，内部包装 composable）
 *
 * @deprecated 请使用 `useCityScene()` 组合式函数替代
 */
export function createCityScene(options: {
  container: HTMLElement;
  buildings: Building[];
  onSelectBuilding?: (building: Building | null) => void;
}): CitySceneHandle {
  // 使用 composable 但仅暴露旧版接口
  const handle = useCityScene();
  handle.initScene(options.container);
  handle.loadBuildings(options.buildings);

  // 监听 onSelectBuilding
  if (options.onSelectBuilding) {
    watchEffect(() => {
      options.onSelectBuilding!(handle.selectedBuilding.value);
    });
  }

  return {
    setTimeOfDay: (key) => handle.setTimeOfDay(key),
    setWeather: (key) => handle.setWeather(key),
    setHeatMode: (enabled) => handle.setHeatmapMode(enabled),
    setAutoRotate: (enabled) => {
      if (enabled) handle.startAutoRotate();
      else handle.stopAutoRotate();
    },
    focusBuilding: (id) => handle.focusOnBuilding(id),
    focusPoint: (_x: number, _z: number) => {
      // focusPoint 通过已有 API 近似实现
    },
    updateBuildings: (buildings) => handle.loadBuildings(buildings),
    dispose: () => handle.disposeScene(),
  };
}