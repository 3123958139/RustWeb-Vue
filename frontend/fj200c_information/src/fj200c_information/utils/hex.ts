/**
 * 十六进制工具函数
 *
 * 提供字节数组与十六进制字符串之间的相互转换。
 * 用于构造命令帧和解析设备返回数据。
 */

/**
 * 字节数组 → 大写十六进制字符串（空格分隔）
 *
 * @param bytes - 字节数组
 * @returns 如 "EB 90 64 00"
 *
 * @example
 * bytesToHex(new Uint8Array([0xEB, 0x90])) // => "EB 90"
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0").toUpperCase())
    .join(" ");
}

/**
 * 十六进制字符串 → 字节数组
 *
 * 支持带空格和不带空格的格式：
 * - "EB 90 64" ✓
 * - "EB9064"   ✓
 *
 * @param hex - 十六进制字符串
 * @returns 字节数组
 * @throws 无效的十六进制字符串（奇数长度或非法字符）
 *
 * @example
 * hexStringToUint8Array("EB 90") // => Uint8Array [0xEB, 0x90]
 */
export function hexStringToUint8Array(hex: string): Uint8Array {
  // 移除非十六进制字符（保留 0-9, a-f, A-F）
  const cleaned = hex.replace(/[^0-9a-fA-F]/g, "");
  if (cleaned.length % 2 !== 0) {
    throw new Error(`无效的十六进制字符串: ${hex}`);
  }
  const result = new Uint8Array(cleaned.length / 2);
  for (let i = 0; i < result.length; i++) {
    result[i] = parseInt(cleaned.substring(i * 2, i * 2 + 2), 16);
  }
  return result;
}

/**
 * 字节数组 → 连续十六进制字符串（无分隔符）
 *
 * 用于校验和对比场景，与 bytesToHex 的区别是没有空格分隔。
 *
 * @param bytes - 字节数组
 * @returns 如 "EB906400"
 */
export function bytesToContinuousHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0").toUpperCase())
    .join("");
}
