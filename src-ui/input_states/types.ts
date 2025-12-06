/**
 * 输入上下文枚举
 * 用于标识当前用户正在与哪个 UI 组件交互
 * 注意：这是 UI 渲染状态，不是业务数据
 */
export enum InputContext {
    /** 主搜索页面 */
    MainSearch = 'main_search',
    /** Everything 搜索页面 */
    Everything = 'everything',
    /** 参数输入模式 */
    ParameterInput = 'parameter',
}
