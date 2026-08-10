/**
 * fw150 角色专属 API（设备台账）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容。
 */
import { getFw150 } from "@shared/api/generated";

/** 台账条目类型（由 OpenAPI 生成，Fw150LedgerItem 别名） */
export type { Fw150LedgerItem as LedgerItem } from "@shared/api/generated";

/** 创建设备台账 API 对象（无需传参，请求自动走 @shared 注入的 Axios 实例） */
export function createFw150Api() {
    return {
        /**
         * 获取设备台账列表
         * @returns 台账条目数组
         */
        async getItems() {
            return getFw150().fw150ListItems();
        },
    };
}

/** 设备台账 API 类型 */
export type Fw150Api = ReturnType<typeof createFw150Api>;
