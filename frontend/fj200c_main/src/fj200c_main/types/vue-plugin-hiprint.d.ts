/**
 * @module types/vue-plugin-hiprint
 * @description 第三方库类型声明
 *
 * 为无 TypeScript 类型定义的 `vue-plugin-hiprint` 库提供模块类型声明。
 * 放置于 `src/types/` 下，TypeScript 自动识别。
 */

declare module 'vue-plugin-hiprint' {
  interface TableColumn {
    title: string
    field?: string
    width: number
    align?: string
    /** 子列（分组表头） */
    columns?: TableColumn[]
  }

  interface PrintElementOptions {
    width: number
    height: number
    top: number
    left: number
    title?: string
    fontSize?: number
    fontWeight?: string
    fontFamily?: string
    textAlign?: string
    color?: string
    field?: string
    /** hiprint 要求二维数组：外层为列组，可表示多行/分组表头 */
    columns?: TableColumn[][]
    cellPadding?: number
    textType?: string
  }

  interface PrintPanel {
    addPrintText(options: { options: PrintElementOptions }): void
    addPrintLongText(options: { options: PrintElementOptions }): void
    addPrintTable(options: { options: PrintElementOptions }): void
  }

  interface PrintTemplateOptions {
    template?: Record<string, unknown>
    settingContainer?: string
    paginationContainer?: string
    history?: boolean
    fontList?: { title: string; value: string }[]
    dataMode?: number
    onDataChanged?: (type: string, json: unknown) => void
    onImageChooseClick?: (target: unknown) => void
  }

  interface HiprintStatic {
    init(options?: { providers?: unknown[]; lang?: string; host?: string; token?: string }): void
    PrintTemplate: new (options?: PrintTemplateOptions) => PrintTemplate
    PrintElementTypeManager: {
      buildByHtml(element: unknown): void
    }
  }

  interface PrintTemplate {
    addPrintPanel(options: { width: number; height: number; paperHeader?: number; paperFooter?: number }): PrintPanel
    design(selector: string): void
    print(data: unknown, options?: unknown, extras?: { styleHandler?: () => string; callback?: () => void }): void
    print2(data: unknown, options?: { printer?: string; title?: string; client?: string }): void
    preview(data: unknown, options?: unknown, extras?: { styleHandler?: () => string }): void
    /** 生成已分页的打印 HTML（jQuery 集合，访问 [0].outerHTML 取整份文档） */
    getHtml(data: unknown, options?: unknown): unknown[]
    on(event: string, handler: (...args: unknown[]) => void): void
    getPrinterList(): { name: string }[]
  }

  export const hiprint: HiprintStatic
  export const hiPrintPlugin: {
    install(app: unknown, pluginName?: string, autoConnect?: boolean): void
    disAutoConnect(): void
  }
  export const defaultElementTypeProvider: new () => unknown
}
