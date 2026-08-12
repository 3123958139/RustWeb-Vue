/**
 * 通信协议生成器（protocol_generator）工具函数
 *
 * 从 demo-protocol（Tauri 桌面应用）src/utils/protocol.ts 迁移：
 * 按数据类型大小自动重排字节范围与序号。
 */
import type { ProtocolField } from '../types/protocol'
import { getTypeSize } from '../types/protocol'

export function recalcFields(fields: ProtocolField[]): ProtocolField[] {
  let offset = 0
  return fields.map((f, i) => {
    const size = getTypeSize(f.dataType)
    let byteRange = ''
    if (size > 0) {
      byteRange = size === 1 ? `${offset}` : `${offset}-${offset + size - 1}`
      offset += size
    } else if (f.length && f.length > 0) {
      byteRange = f.length === 1 ? `${offset}` : `${offset}-${offset + f.length - 1}`
      offset += f.length
    } else {
      byteRange = `${offset}~N`
    }
    return { ...f, index: i + 1, byteRange }
  })
}