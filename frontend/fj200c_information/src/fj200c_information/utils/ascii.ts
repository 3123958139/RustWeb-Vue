/**
 * ASCII 工具函数
 *
 * 提供 ASCII 字符串与小端序字节数组之间的转换。
 * 用于构造命令帧中的 ASCII 数据区（如产品名称、编号等）。
 */

/**
 * ASCII 字符串 → 小端序 8 字节数组
 *
 * 转换规则：
 * - 字符按从右到左的顺序填入字节数组（小端序）
 * - 不足 8 个字符时，高位（数组开头）用空格 0x20 填充
 * - 与后端 little_endian_bytes_to_ascii 函数互逆
 *
 * @param input - 输入字符串，最多 8 个 ASCII 字符
 * @returns 转换成功的 8 字节数组，或错误信息字符串
 *
 * @example
 * asciiToLittleEndianBytes("ABC") // => Uint8Array [0x20, 0x20, 0x20, 0x20, 0x20, 0x43, 0x42, 0x41]
 * asciiToLittleEndianBytes("12345678") // => Uint8Array [0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31]
 */
export function asciiToLittleEndianBytes(input: string): Uint8Array | string {
  if (input.length > 8) {
    return `输入字符串长度超过 8 个字符，当前长度：${input.length}`;
  }

  const bytes: number[] = [];

  for (let i = 0; i < input.length; i++) {
    const charCode = input.charCodeAt(i);
    if (charCode > 127) {
      return `非 ASCII 字符：0x${charCode.toString(16).padStart(2, " ")}`;
    }
    bytes.push(charCode);
  }

  // 创建 8 字节数组，初始填充空格（0x20）
  const result = new Uint8Array(8).fill(0x20);
  // 从右到左填入字符（小端序：低位在前）
  for (let i = 0; i < bytes.length; i++) {
    result[result.length - 1 - i] = bytes[i];
  }

  return result;
}
