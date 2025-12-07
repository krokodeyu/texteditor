## 1. 整体目标

做的是一个**命令行多文件文本编辑器**，但重点不在功能多，而是在于：

- 分清 **接口层 / 领域层 / 基础设施**；

- 用上 **Command / Memento / Observer** 等模式；

- 让“新增命令”只改少量代码，符合 SOLID。

---

## 2. 分层架构

### 2.1 接口层：Application + Router + Outcome

**Application**

- 负责 REPL 循环：
  
  `读一行命令 → 用 router 解析 → 执行 handler → 打印结果 / 发事件`

- 核心流程：
  
  1. `router.resolve(line)`：只读地解析，得到 `(handler, args)`；
  
  2. 再用可变借用调用 `handler(self, &args)`；
  
  3. handler 返回 `Outcome`：
     
     `struct Outcome {     print: Option<String>,     log:   Option<String>, // 命令日志内容     exit:  bool, }`
  
  4. Application 统一处理：
     
     - `print` → `println!`
     
     - `log` → 发布 `Event::Command{ file, cmd }` 到总线
     
     - `exit == true` → `save_workspace_memento()` 并退出循环

- 启动 / 退出：
  
  - 启动时尝试从 `.editor_workspace` 恢复 `WorkspaceMemento`；
  
  - 退出时保存 memento（现在放在 `work_dir/.editor_workspace`），打印：
    
    `[info] workspace saved to "work_dir\\.editor_workspace"`

**Router + Commands**

- 有一个 `commands` 模块，里面定义：
  
  `pub type Handler = fn(&mut Application, &[String]) -> AppResult<Outcome>;  pub struct CommandDef {    pub name: &'static str,    pub handler: Handler, }  pub static COMMANDS: &[CommandDef] = &[     LOAD_COMMAND,     APPEND_COMMAND,     SAVE_COMMAND,     EDIT_COMMAND,    // ... ];`

- `Router::new()` 启动时遍历 `COMMANDS`，把 `name -> handler` 塞进 HashMap。

- 新增命令 = **加一个文件 + 写一个 `cmd_xxx` 函数 + 往 COMMANDS 里挂一项**，  
  Application 不用改。

---

## 3. 领域层：Workspace + Editor + DocCommand

### 3.1 Workspace：多文件 + 工作区快照

- 内部结构：
  
  `pub struct Workspace {     base_dir: PathBuf,                    // 统一的 work_dir     editors:  HashMap<PathBuf, Editor>,   // 打开的文件     active:   Option<PathBuf>,            // 当前活动文件 }`

- 职责：
  
  - **路径解析**：`resolve_path(Some("a.txt"))` → `base_dir/a.txt`；
  
  - 多文件管理：`load` / `edit` / `close` / `editor-list`；
  
  - 持久化：
    
    - `to_memento()`：只记打开了哪些文件、active 是谁、各自的 `modified` / `logging` 标志；
    
    - `from_memento()`：根据 memento 中的路径重新打开文件。
  
  - 保存：
    
    - `save_file(path)`：找到对应 Editor，调用 `ed.save_to(path)`；
    
    - `save_all()`：遍历 `editors` 保存全部；
  
  - 文档命令执行：
    
    - `exec_doc(Box<dyn DocCommand>)`：
      
      - 找到当前活动 `Editor`；
      
      - 委托给 `Editor::exec_doc(cmd)`；
      
      - 触发 undo/redo 栈更新。

### 3.2 Editor：行数组 + 命令栈

- 存储结构：
  
  `pub struct Editor {     lines: Vec<String>,     modified: bool,     logging: bool,     undo_stack: Vec<Box<dyn DocCommand>>,     redo_stack: Vec<Box<dyn DocCommand>>, }`

- 基本操作：
  
  - `append_line(&str)`
  
  - `insert_text(line, col, text)`
  
  - `delete_text(line, col, len)`
  
  - `peek_text(line, col, len)`
  
  - `show(start, end)`，返回带行号的字符串
  
  - `save_to(path)`：把整文件写回磁盘并清除 `modified`

- 命令模式（Command + Undo/Redo）：
  
  `pub trait DocCommand {    fn execute(&mut self, ed: &mut Editor) -> AppResult<()>;    fn undo(&mut self, ed: &mut Editor) -> AppResult<()>; }`
  
  示例：`AppendLineCommand`
  
  - `execute`：
    
    - 记录插入前的 `line_index`；
    
    - 调用 `ed.append_line()`。
  
  - `undo`：
    
    - 调用 `ed.pop_line()`。

  Editor 内部对 doc command 的管理：

- `exec_doc(cmd)`：
  
  - 清理 redo 栈；
  
  - `cmd.execute(self)?`;
  
  - push 到 `undo_stack`；
  
  - `modified = true`。

- `undo()`：
  
  - 从 `undo_stack` pop；
  
  - 调 `cmd.undo(self)?`；
  
  - push 到 `redo_stack`。

- `redo()`：
  
  - 从 `redo_stack` pop；
  
  - 再调用 `execute`；
  
  - 回到 `undo_stack`。

---

## 4. 基础设施：事件总线 + 日志 + memento + 错误

### 4.1 EventBus + Event（Observer）

- `Event` 定义：
  
  `pub enum Event {     SessionStart,     Command { file: Option<PathBuf>, cmd: String },     Error { code: i32, message: String }, }`

- `Subscriber: Send`：
  
  `pub trait Subscriber: Send {    fn on_event(&mut self, event: &Event); }`

- `EventBus`：
  
  - 持有一堆 `Box<dyn Subscriber>`；
  
  - `publish(&Event)` 时按顺序调用每个 subscriber 的 `on_event`。

- `Logger` 作为 Subscriber：
  
  - 订阅 `Event::SessionStart` / `Event::Command` / `Event::Error`；
  
  - 根据 `file` 决定写 `work_dir/.<filename>.log`；
  
  - 所有错误统一写到 `work_dir/.app.log`；
  
  - 每个日志文件首次写入时，加“session start at …”头；
  
  - 任意 I/O 错误只 `eprintln!("[warn] ...")`，不影响主流程。

### 4.2 持久化：Memento + JSON

- `WorkspaceMemento`：
  
  `pub struct WorkspaceMemento {     open_files: HashMap<PathBuf, FileFlags>,     active: Option<PathBuf>, }  pub struct FileFlags {     modified: bool,     logging: bool, }`

- 使用 `serde` / `serde_json` 将 memento 写入 `work_dir/.editor_workspace`。

- Application 启动时若检测到该文件存在，就 `load` + `workspace.from_memento(m)`，并打印一条 info。

### 4.3 错误：AppError + AppResult

- `AppError` 用 `thiserror` 实现，统一错误类型：
  
  - 包含 I/O、参数错误（`InvalidArgs`）、内部逻辑错误等。

- `AppResult<T> = Result<T, AppError>`。

- `AppError::report()` 负责向用户友好打印（带 `[error]` 前缀）。

- `Application::publish_error(e)`：
  
  1. `e.report();`
  
  2. 发布 `Event::Error { code: e.code(), message: e.to_string() }`。

---

## 5. 测试策略

### 5.1 Rust 单元测试

- Editor 层：
  
  - `append_line`/`pop_line`；
  
  - `insert_text`/`delete_text` 的越界判断；
  
  - `DocCommand` + undo/redo 栈行为。

- Workspace 层：
  
  - `resolve_path` 正确处理 `base_dir` 和绝对路径；
  
  - `save_file` / `save_all` 真正写到 `work_dir`；
  
  - `log_show` 能读到 `work_dir/.filename.log`。

- Application 层：
  
  - `save_workspace_memento` 写出 `.editor_workspace`；
  
  - 成功命令 → `Event::Command`；
  
  - `publish_error` → `Event::Error`。

### 5.2 用户层随机测试（Fuzz）

- 用 Python 写了 `tests/random_cli_fuzz.py`，逻辑大致是：
  
  1. 先 `cargo build`；
  
  2. 建 `fuzz_runs/run_X` 作为工作目录；
  
  3. 每个 session 随机生成一串命令（load/append/init/save/editor-list/undo/redo/dir-tree/log-* 以及垃圾命令），最后自动加 `exit`；
  
  4. 子进程运行二进制，收 stdout / stderr / exit code；
  
  5. 把命令和输出写到 `fuzz_runs/session_X.log`，人工抽查；
  
  6. 自动检查：不崩溃（exit==0），stderr 没有 panic。
