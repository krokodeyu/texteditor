#set text(
  font: ("Consolas", "SimSun"),
)
#show raw.where(block: false): it => text(
  font: ("Consolas", "KaiTi"),
  it,
)
#show raw.where(block: true): it => text(
  font: ("Consolas", "KaiTi"),
  it,
)




= 命令行多文件文本编辑器设计文档

== 1. 整体目标

做的是一个 *命令行多文件文本编辑器*，但重点不在功能多，而是在于：

- 分清 *接口层 / 领域层 / 基础设施*；
- 用上 *Command / Memento / Observer* 等模式；
- 让“新增命令”只改少量代码，符合 SOLID。

== 2. 分层架构

=== 2.1 接口层：Application + Router + Outcome

*Application*

- 负责 REPL 循环：

  `读一行命令 → 用 router 解析 → 执行 handler → 打印结果 / 发事件`

- 核心流程：

  1. `router.resolve(line)`：只读地解析，得到 `(handler, args)`；
  2. 再用可变借用调用 `handler(self, &args)`；
  3. handler 返回 `Outcome`：

     ```rust
     struct Outcome {
       print: Option<String>,
       log:   Option<String>, // 命令日志内容
       exit:  bool,
     }
     ```

  4. Application 统一处理：

     - `print` → `println!`
     - `log` → 发布 `Event::Command{ file, cmd }` 到总线
     - `exit == true` → `save_workspace_memento()` 并退出循环

- 启动 / 退出：

  - 启动时尝试从 `.editor_workspace` 恢复 `WorkspaceMemento`；
  - 退出时保存 memento（现在放在 `work_dir/.editor_workspace`），打印：

    `[info] workspace saved to "work_dir/.editor_workspace"`

*Router + Commands*

- 有一个 `commands` 模块，里面定义：

  ```rust
  pub type Handler = fn(&mut Application, &[String]) -> AppResult<Outcome>;
  pub struct CommandDef {
    pub name: &'static str,
    pub handler: Handler,
  }
  
  pub static COMMANDS: &[CommandDef] = &[
    LOAD_COMMAND,
    APPEND_COMMAND,
    SAVE_COMMAND,
    EDIT_COMMAND,
    // ...
  ];
```