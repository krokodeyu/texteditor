# Lab2 架构设计文档

## 0. 概述

本项目在 Lab1 多文件文本编辑器基础上增量扩展，新增 XML 编辑器、编辑时长统计模块、拼写检查模块；并修改 init 与 editor-list 两个命令。Lab2 的评分主要关注新增/修改模块的架构设计、第三方依赖管理与可测试性。

Lab2 必须实现（2 改 + 7 新）共 9 个命令：

- 修改：`init`、`editor-list`
- 新增：`insert-before`、`append-child`、`edit-id`、`edit-text`、`delete-element`、`xml-tree`、`spell-check`

---

## 1. 系统架构

### 1.1 模块划分

- **Application（程序入口 / 运行循环）**
  
  - 负责 REPL：读取用户输入 -> 路由 -> 执行命令 -> 输出 Outcome
  - 统一发布事件（EventBus），供横切模块监听（统计、日志等）

- **Commands（命令层）**
  
  - 每个命令独立 handler，返回 `Outcome { print, log, exit }`
  - 命令层不直接依赖第三方拼写检查库，而依赖 `SpellChecker` 抽象接口（见 1.3）

- **Workspace（工作区）**
  
  - 管理打开文件与当前活动文件
  - 提供“读取内容/保存/展示/根据文件类型分派 editor”的统一 API
  - 持久化 memento：保存打开文件列表、活动文件等（会话重启恢复）

- **Editors（编辑器层）**
  
  - `TextEditor`：行级编辑、show、undo/redo（Lab1）
  - `XmlEditor`：DOM 树编辑、id->node 映射、xml-tree 展示、undo/redo（Lab2）
  - 统一通过 `EditorInstance`（枚举/trait 对象）在 Workspace 内多态分派

- **Undo/Redo（命令模式）**
  
  - 每个“修改型操作”对应一个可回放对象（Do/Undo）
  - “显示类命令”不进入撤销栈（xml-tree/spell-check 等）

- **Statistics（编辑时长统计，横切）**
  
  - 会话内统计“每个文件成为活动文件的累计时长”，显示在 `editor-list` 中
  - 失败只提示 warn，不影响主流程

- **SpellCheck（拼写检查，横切）**
  
  - 通过适配器封装第三方服务（如 LanguageTool）
  - 支持 txt 全文本；xml 仅元素文本，不检查标签/属性

### 1.2 模块依赖关系

![](C:\Users\11460\AppData\Roaming\marktext\images\2025-12-15-21-27-56-image.png)

---

## 2. 核心设计与模式应用

### 2.1 多态与接口设计（满足“文本/XML 编辑器”要求）

- Workspace 内部持有 `HashMap<PathBuf, EditorInstance>`
- `EditorInstance` 对外暴露：
  - `kind()`（Text / Xml）
  - `as_text()/as_xml()` 或统一的 `show()/save()/apply_cmd()` 等方法（按你的实现）
- Commands 层只调用 Workspace API，不直接操作具体 Editor，保证模块边界清晰（便于测试与替换）。

### 2.2 XML 编辑器设计（Composite + Adapter + Command）

- **数据结构**：解析 XML 为 DOM 树，并维护 `id -> element` 映射，便于 O(1) 定位节点
- **约束**：
  - XML 声明必须在首行；每个元素必须有唯一 id；不支持混合内容
  - XML 文件不支持 `# log`，改为根元素属性 `log="true"` 代表启用日志
- **命令**：insert-before/append-child/edit-id/edit-text/delete-element/xml-tree（显示类命令不入撤销栈）

### 2.3 统计模块设计（Observer + Decorator）

- **计时规则**：
  - 开始：文件成为活动文件（load/edit）
  - 停止：切换文件/关闭/退出
  - 再次 load：时长重置为 0
- **展示**：editor-list 输出文件名后附带时长（可读格式：秒/分钟/小时/天）
- **解耦**：统计模块作为 Subscriber 监听 EventBus；失败仅 warn，不影响功能继续执行

### 2.4 拼写检查模块设计（Adapter + DI + Mock）

- 需求强调“架构设计与第三方库管理能力”，而非算法；在线 API 需考虑网络异常；失败仅 warn，不影响其他功能

- 关键点：
  
  1) **依赖隔离**：第三方库仅出现在 adapter 内  
  2) **接口抽象**：编辑器依赖接口而非实现
  3) **依赖注入**：Checker 由 Application 构造后注入
  4) **可测试性**：用 MockChecker 测试，无需真实网络

- 文件范围：
  
  - txt：检查全部文本
  - xml：仅检查元素文本内容，不检查标签/属性

---

## 3. 运行说明

### 3.1 环境

- Rust：stable（建议 1.7x+）
- 编码：全项目统一 UTF-8（实验要求）

### 3.2 构建

- `cargo build`

### 3.3 运行

- `cargo run`

### 3.4 运行测试

- Rust 自动化测试：`cargo test`
- 用户层随机测试：`python tests/random_test.py`
