/**
 * 城市 3D 场景自定义 GLSL 着色器
 *
 * - `sky`：天空穹顶渐变（顶/底双色 + 太阳/月亮辉光点）
 * - `ground`：地面圆盘（径向渐变 + 外缘呼吸辉光环）
 */

export const skyVertexShader = /* glsl */ `
varying vec3 vWorldPos;
void main() {
  vWorldPos = (modelMatrix * vec4(position, 1.0)).xyz;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

export const skyFragmentShader = /* glsl */ `
uniform vec3 uTopColor;
uniform vec3 uBottomColor;
uniform vec3 uSunColor;
uniform float uSunIntensity;
uniform vec3 uSunDir;
uniform vec3 uMoonColor;
uniform float uMoonIntensity;
uniform vec3 uMoonDir;
varying vec3 vWorldPos;

void main() {
  vec3 dir = normalize(vWorldPos);
  float h = clamp(dir.y * 0.5 + 0.5, 0.0, 1.0);

  // 垂直渐变：底部 → 顶部
  vec3 color = mix(uBottomColor, uTopColor, pow(h, 1.25));

  // 太阳辉光（明亮核 + 柔和光晕）
  float sunDot = clamp(dot(dir, normalize(uSunDir)), 0.0, 1.0);
  color += uSunColor * uSunIntensity * pow(sunDot, 28.0);
  color += uSunColor * uSunIntensity * 0.4 * pow(sunDot, 6.0);

  // 月亮辉光
  float moonDot = clamp(dot(dir, normalize(uMoonDir)), 0.0, 1.0);
  color += uMoonColor * uMoonIntensity * pow(moonDot, 16.0);

  gl_FragColor = vec4(color, 1.0);
}
`;

export const groundVertexShader = /* glsl */ `
varying vec3 vWorldPos;
void main() {
  vWorldPos = (modelMatrix * vec4(position, 1.0)).xyz;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

export const groundFragmentShader = /* glsl */ `
uniform vec3 uBaseColor;
uniform vec3 uGlowColor;
uniform float uTime;
uniform float uRadius;
varying vec3 vWorldPos;

void main() {
  float dist = length(vWorldPos.xz);

  // 径向渐变（中心略亮）
  float center = 1.0 - smoothstep(0.0, uRadius * 0.55, dist);

  // 外缘辉光（呼吸动画）
  float edge = smoothstep(uRadius * 0.86, uRadius, dist);
  float pulse = 0.55 + 0.45 * sin(uTime * 1.4 - dist * 0.02);

  // 扫描涟漪（从中心向外扩散）
  float ripple = 0.5 + 0.5 * sin(uTime * 0.8 - dist * 0.045);
  float rippleMask = 1.0 - smoothstep(uRadius * 0.1, uRadius * 0.92, abs(fract(uTime * 0.16 + dist / 46.0) * 46.0 - 23.0));

  vec3 color = uBaseColor;
  color += uGlowColor * (0.06 * center + 0.55 * edge * pulse + 0.10 * rippleMask * ripple);

  // 圆盘外缘淡出
  float alpha = 1.0 - smoothstep(uRadius * 0.965, uRadius, dist);
  gl_FragColor = vec4(color, alpha);
}
`;
