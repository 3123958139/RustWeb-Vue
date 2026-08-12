/**
 * 通信协议生成器（protocol_generator）类型定义
 *
 * 从 demo-protocol（Tauri 桌面应用）src/types/protocol.ts 迁移。
 */

/** 协议字段（通信协议表一行） */
export interface ProtocolField {
  index: number
  byteRange: string
  name: string
  unit: string
  dataType: string
  length?: number
  remark: string
}

/** C# 数据类型定义（label + 固定大小） */
export interface CSharpType {
  label: string
  size: number
}

/** 参数表条目（CSV 参数表一行） */
export interface CsvParameter {
  name: string
  alias: string
  unit: string
  dataType: string
  remark: string
}

/** C# 内置类型及其大小（0 = 可变长） */
export const CSharpTypes: CSharpType[] = [
  { label: 'bool', size: 1 },
  { label: 'byte', size: 1 },
  { label: 'sbyte', size: 1 },
  { label: 'short', size: 2 },
  { label: 'ushort', size: 2 },
  { label: 'int', size: 4 },
  { label: 'uint', size: 4 },
  { label: 'long', size: 8 },
  { label: 'ulong', size: 8 },
  { label: 'float', size: 4 },
  { label: 'double', size: 8 },
  { label: 'decimal', size: 16 },
  { label: 'char', size: 2 },
  { label: 'string', size: 0 },
  { label: 'byte[]', size: 0 },
]

/** 获取类型固定大小（未知类型返回 0 = 需要手动指定长度） */
export function getTypeSize(typeName: string): number {
  const t = CSharpTypes.find(t => t.label === typeName)
  return t ? t.size : 0
}