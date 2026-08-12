/**
 * protocol_generator 角色专属 API（通信协议生成）
 *
 * 基于 orval 生成的客户端（@shared/api/generated）组装 facade；
 * Excel 二进制下载走原始 axios 实例（responseType: blob）。
 */
import { getProtocolGenerator } from "@shared/api/generated";
import { api } from "@/api";
import type { CsvParameter, ProtocolField } from "../types/protocol";

const generated = getProtocolGenerator();

/** 协议导出请求体（Markdown / Excel 共用，与后端 Schema 一致） */
export interface ProtocolExportRequest {
  title: string;
  data: ProtocolField[];
}

/**
 * 触发浏览器下载 Blob（协议 JSON / Excel / CSV 通用）
 */
export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/** 创建协议生成器 API 对象 */
export function createProtocolGeneratorApi() {
  return {
    /** 读取默认参数表（后端首次自动写入种子内容） */
    async getDefaultCsv() {
      return generated.protocolGeneratorGetDefaultCsv();
    },

    /** 保存默认参数表（服务器 parameters.csv，UTF-8 BOM） */
    async saveDefaultCsv(data: CsvParameter[]) {
      return generated.protocolGeneratorSaveDefaultCsv(data);
    },

    /** 导出协议表 Markdown 文本 */
    async exportMarkdown(req: ProtocolExportRequest) {
      return generated.protocolGeneratorExportMarkdown(req);
    },

    /** 导出协议表 Excel 文件（后端生成 xlsx 二进制 → Blob 下载） */
    async exportExcel(req: ProtocolExportRequest, filename: string) {
      try {
        const resp = await api.post<Blob>("/protocol_generator/excel", req, {
          responseType: "blob",
        });
        downloadBlob(new Blob([resp.data], { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" }), filename);
      } catch (e: any) {
        throw new Error(e?.response?.data ? "导出失败（服务器未返回 xlsx）" : e);
      }
    },

    /** 解析 CSV 文本（前端上传文件内容）为参数表 */
    async parseCsv(content: string) {
      return generated.protocolGeneratorParseCsv({ content });
    },

    /** 参数表序列化为 CSV 文本（UTF-8 BOM） */
    async serializeCsv(data: CsvParameter[]) {
      return generated.protocolGeneratorSerializeCsv(data);
    },

    /** 下载协议表 JSON 文件（浏览器端直接生成，不需后端） */
    downloadProtocolJson(data: ProtocolField[], filename: string) {
      downloadBlob(
        new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
        filename,
      );
    },

    /** 上传解析协议表 JSON 文件（浏览器端直接解析，不需后端） */
    parseProtocolJsonFile(file: File): Promise<ProtocolField[]> {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          try {
            resolve(JSON.parse(String(reader.result)) as ProtocolField[]);
          } catch (e) {
            reject(e);
          }
        };
        reader.onerror = () => reject(reader.error);
        reader.readAsText(file);
      });
    },
  };
}

/** 协议生成器 API 类型 */
export type ProtocolGeneratorApi = ReturnType<typeof createProtocolGeneratorApi>;