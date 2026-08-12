/**
 * 通信协议生成应用（protocol_generator）类型定义
 *
 * 业务类型统一由 @shared 公共包导出；
 * 后端返回的 DTO 由 orval 从 OpenAPI 规范生成（@shared/api/generated），
 * 修改后端结构体后运行 `npm run gen:api` 重新生成。
 *
 * 协议编辑器内部类型（ProtocolField / CSharpTypes 等）位于
 * @/protocol_generator/types/protocol。
 */
export * from "@shared/types";