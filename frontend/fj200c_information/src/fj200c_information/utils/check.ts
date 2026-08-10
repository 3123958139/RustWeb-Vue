/**
 * 校验和计算工具函数
 *
 * 提供帧数据校验功能，与后端 frame_validator 保持一致。
 * 使用累加和取模算法（所有字节求和后对 256 取模）。
 */

/**
 * 计算字节数组的累加和校验值
 *
 * 算法：所有字节累加后对 256 取模（即取低 8 位）。
 * 与后端 Rust 实现一致。
 *
 * @param bytes - 待校验的字节数组
 * @returns 校验和（0~255）
 *
 * @example
 * calculateChecksum(new Uint8Array([0xEB, 0x90, 0x64])) // => (0xEB + 0x90 + 0x64) % 256
 */
export function calculateChecksum(bytes: Uint8Array): number {
  let sum = 0;
  for (const b of bytes) {
    sum = (sum + b) % 256;
  }
  return sum;
}

/**
 * 计算 100 字节帧的完整校验
 *
 * 前 99 字节累加和取模，第 100 字节为校验位。
 * 如果帧长度不足 100 字节，返回 0。
 *
 * @param frame - 100 字节帧数据
 * @returns 校验和
 */
export function frameChecksum(frame: Uint8Array): number {
  if (frame.length < 100) {
    return 0;
  }
  let sum = 0;
  for (let i = 0; i < 99; i++) {
    sum = (sum + frame[i]) % 256;
  }
  return sum;
}
