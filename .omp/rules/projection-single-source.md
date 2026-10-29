---
name: projection-single-source
description: 投影类型（XxxInfo/XxxSnapshot/XxxDto/XxxRepr）必须由权威类型 Xxx 经唯一薄构造派生，禁止复制权威已有的查询与组装逻辑
condition:
  - 'struct\s+\w+(Info|Snapshot|Dto|Repr)\b'
  - 'fn\s+to_(info|snapshot|dto)\b'
  - '\b\w+(Info|Snapshot|Dto)\s*\{'
  - 'impl\s+Serialize\s+for'
  - 'export\s+(interface|type)\s+\w+(Info|Snapshot|Dto)\b'
scope:
  - 'tool:edit(crates/**/*.rs)'
  - 'tool:write(crates/**/*.rs)'
  - 'tool:edit(src-tauri/**/*.rs)'
  - 'tool:write(src-tauri/**/*.rs)'
  - 'tool:edit(src-ui/**/*.ts)'
  - 'tool:write(src-ui/**/*.ts)'
---

# 投影类型：唯一权威 + 薄构造

`Xxx` 是权威类型；`XxxInfo` / `XxxSnapshot` / `XxxDto` 只是它的投影。投影**不是**第二份真相。

## 派生方式

- **唯一构造入口**：在权威类型旁提供 `impl Xxx { pub fn to_info(&self) -> XxxInfo }`，字段逐一从 `self` 读取。禁止命令层、视图层、其他模块各写一套组装。
- **查询与搜索复用权威实现**：取列表/详情/快照时先调用 `Xxx` 已有的函数（扫描、按 id 定位、过滤、统计），再 `.to_info()` 收尾。禁止为投影复制一份查询、扫描或解析逻辑。
- **投影只做形状裁剪**：字段子集、camelCase 键名、脱敏。不能引入权威类型里不存在的事实，也不能在投影里重算权威已经算出的值。
- **一个投影一个构造**：同一投影不得出现列表版、详情版两个构造函数；字段不同就扩展该构造或另取明确的投影名。

## 什么时候允许存在投影类型

只有买到一项能力时才允许定义投影，否则直接用权威类型：

- 字段裁剪或脱敏（权威类型含不应对外的字段）
- 跨 crate / 发布包的 semver 边界（公开类型不能依赖内部 crate）
- 协议版本隔离（下发改动不能牵动内部实现）
- 入口白名单（不可信输入必须逐项放行，如历史消息只接受 `user` / `assistant`）

即便成立，投影仍放在权威类型旁、仍只有一个薄构造入口。

## 序列化代理（`XxxRepr`）与动作结果

- serde 表达不了所需形状（`flatten` + `tag`、变体改名）时，允许私有 `XxxRepr` 作形状代理；但自定义 `Serialize` / `Deserialize` **必须** 委托给它的 `From` / `Into`，不得再写一份 match 映射。
- 配置动作与命令结果里的实体数据 **必须** 用现有类型或命名投影序列化；禁止用 `json!` 逐字段拼出已有类型的形状。外层单键包装（如 `{ "models": entries }`）不受此限。

## 反例

- 权威类型已有 `from_metadata` 构造，命令层又按同样字段拼一遍。
- 为 `XxxDto` 写 `impl From<Xxx>` 逐字段搬运，且两者字段完全一致——没买到任何能力的隧道层。
- 列表扫一遍磁盘、详情再扫一遍，只为产出两个投影。
- 已有配置类型却用 `json!({ "name": …, "path": … })` 手拼同类形状下发给前端。

## 正例

- 播放列表项：`playlist_tracks(id)` 返回权威记录，投影只做 `to_info()`。
- 插件行：宿主按已解析元数据构造唯一投影，列表、详情、安装返回值共用它。
