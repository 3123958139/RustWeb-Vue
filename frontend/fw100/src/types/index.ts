/**
 * 设备台账应用（fw100）类型定义
 *
 * 业务类型统一由 @shared 公共包导出；
 * 后端返回的 DTO（LedgerItem 等）由 orval 从 OpenAPI 规范生成（@shared/api/generated），
 * 修改后端结构体后运行 `npm run gen:api` 重新生成。
 */
export * from "@shared/types";
export type { LedgerItem } from "@shared/api/generated";
