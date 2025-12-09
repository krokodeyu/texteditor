//! 工作区
//! 管理多文件上下文、状态持久化。

use std::{
    collections::HashMap,
    collections::hash_map::Entry,
    fs, 
    io,
    path::{Path, PathBuf},
    fmt::Write,
};


use crate::{
    commands::{doc_command::DocCommand, xml_command::XmlCommand}, 
    editor_instance::{EditorInstance, EditorKind}, 
    error::{AppError, AppResult}, 
    persist::{FileFlags, WorkspaceMemento}, 
    text_editor::TextEditor, 
    xml_editor::XmlEditor,
};

#[derive(Default)]
pub struct Workspace {
    editors: HashMap<PathBuf, EditorInstance>,
    active: Option<PathBuf>,
    base_dir: PathBuf,
}

impl Workspace {
    // 默认生成逻辑
    pub fn default() -> Self {
        let base = PathBuf::from("work_dir");
        let base_d = if !base.exists() {
            fs::create_dir_all(&base).ok();
            base
        } else {
            base
        };
        Self {
            editors: HashMap::new(),
            active: None,
            base_dir: base_d,
        }
    }

    /// 用于处理需要undo的函数。
    pub fn exec_doc(&mut self, cmd: Box<dyn DocCommand>) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?.as_text_mut()?;
        ed.exec_doc(cmd)
    }

    pub fn exec_xml(&mut self, cmd: Box<dyn XmlCommand>) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?.as_xml_mut()?;
        ed.exec_xml(cmd)
    }

    pub fn undo(&mut self) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?;
        ed.undo()
    }

    pub fn redo(&mut self) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?;
        ed.redo()
    }

    // 以下为不需要undo的函数。

    //  文件处理函数
    /// 初始化文件，如果文件已存在，直接返回错误。
    pub fn init(&mut self, kind: EditorKind,i_path: impl AsRef<Path>, i_logging: bool) -> AppResult<()> {
        let path: &Path = i_path.as_ref();
        let key: PathBuf = path.to_path_buf();

        if self.editors.contains_key(&key) {
            return Err(AppError::InvalidArgs("file already exists!".into()));
        }

        //let mut ed: TextEditor = TextEditor::default();
        //if i_logging {
        //    ed.set_logging(true);
        //    ed.append_line("# log");
        //}
        
        let instance = match kind {
            EditorKind::Text => {
                let mut ed: TextEditor = TextEditor::default();
                if i_logging {
                    ed.set_logging(true);
                    ed.append_line("# log");
                }
                EditorInstance::Text(ed)
            },
            EditorKind::Xml => {
                let ed: XmlEditor = XmlEditor::new_with_log(i_logging);
                EditorInstance::Xml(ed)
            }
        };

        self.editors.insert(path.to_path_buf(), instance);
        self.active = Some(key);

        Ok(())
    }

    // 用AsRef<Path>，调用方可传入多种类型。
    /// 加载文件。
    pub fn load(&mut self, i_path: impl AsRef<Path>) -> AppResult<()> {
        let path: &Path = i_path.as_ref();
        let key: PathBuf = path.to_path_buf();

        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(AppError::Io(e)),
        };

        match self.editors.entry(key.clone()) {
            // 已经有这个 editor：不重新载入，只切换 active
            Entry::Occupied(_occupied) => {
                // 如果你之后想加“刷新内容”，可以在这里对已有 editor 调用 load_from
                self.active = Some(key);
            }

            // 没有这个 editor：需要新建一个，可能失败，所以要 ?
            Entry::Vacant(vacant) => {
                let editor = Self::create_editor_for_path(path, &content)?;  // 👈 这里用 ?
                vacant.insert(editor);
                self.active = Some(key);
            }
        }
        Ok(())
    }

    pub fn edit(&mut self, i_path: impl AsRef<Path>) -> AppResult<()> {
        let path: &Path = i_path.as_ref();
        let key: PathBuf = path.to_path_buf();
        if self.editors.contains_key(&key) {
            self.active = Some(key);
        } else {
            return Err(AppError::InvalidArgs("target file hasn't been opened".into()));
        }
        Ok(())
    }

    pub fn close(&mut self) -> AppResult<()>{
        if let Some(active_path) = self.active.take() {
            self.editors.remove(&active_path);
        } else {
            return Err(AppError::InternalError("no file to be closed".into()));
        }
        Ok(())
    }

    pub fn show(&self, start: Option<usize>, end: Option<usize>) -> AppResult<String> {
        let active = self
            .active
            .clone()
            .ok_or_else(|| AppError::InternalError("no active file".into()))?;

        let ed = self
            .editors
            .get(&active)
            .ok_or_else(|| AppError::InternalError("couldn't open active file".into()))?
            .as_text()?;

        let n = ed.count_lines();
        if n == 0 {
            return Ok("<empty>".to_string());
        }
        let s = start.unwrap_or(1).clamp(1, n);
        let e = end.unwrap_or(n).clamp(1,n);
        if e < s {
            return Err(AppError::InvalidArgs(format!("invalid range: {}..{}", s, e)));
        }
        Ok(ed.show(s, e))
    }

    pub fn show_xml_tree(&self, p: impl AsRef<Path>) -> AppResult<String> {
        let path: &Path = p.as_ref();
        let key: PathBuf = path.to_path_buf();

        if !self.editors.contains_key(&key) {
            return Err(AppError::InvalidArgs("no such file!".into()));
        }

        let ed = self.editors
            .get(&key)
            .ok_or_else(|| AppError::InternalError("can't access file".into()))?
            .as_xml()?;

        ed.show_all()
    }

    pub fn list(&self) -> AppResult<String> {
        let mut editor_list: String = String::new();
        for (path, editor) in &self.editors {
            let is_active: bool = self.is_active_equal_to(path);
            let modified: bool = editor.is_modified();
            let line = Self::write_editor(path, is_active, modified);
            let _ = writeln!(&mut editor_list, "{}", line);
        }
        Ok(editor_list)
    }

    pub fn save_file(&mut self, path: impl AsRef<Path>) -> AppResult<()> {
        let p = path.as_ref();
        let key: PathBuf = p.to_path_buf();

        let ed = self
            .editors
            .get_mut(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such path".into()))?;

        ed.save_to(p)?;
        Ok(())
    }

    pub fn save_all(&mut self) -> AppResult<()> {
        for (p, ed) in self.editors.iter_mut() {
            ed.save_to(p)?;
        }
        Ok(())
    }

    pub fn active_file_path(&self) -> Option<PathBuf> {
        self.active.clone()
    }
    
    pub fn has_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_modified(&self) -> Option<bool> {
        self.active
            .as_ref()  // 获取 Option<&PathBuf>
            .and_then(|path| self.editors.get(path))  // 获取 Option<&Editor>
            .map(|editor| editor.is_modified())  // 提取 modified 字段
    }

    pub fn from_memento(&mut self, m: WorkspaceMemento) -> AppResult<()> {
        self.editors.clear();
        self.active = None;

        for (path_str, flags) in m.open_files {
            let path = PathBuf::from(&path_str);
            let content = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(AppError::Io(e)),
            };

            let mut instance = match flags.kind {
                EditorKind::Text => {
                    let mut ed = TextEditor::new();
                    ed.load_from(&content);
                    EditorInstance::Text(ed)
                }
                EditorKind::Xml => {
                    let mut ed = XmlEditor::new();
                    ed.load_from(&content)?;
                    EditorInstance::Xml(ed)
                }
            };

            instance.set_modified(flags.modified);
            instance.set_logging(flags.logging);

            self.editors.insert(path, instance);
        }

        if let Some(active_str) = m.active {
            let active_path = PathBuf::from(&active_str);
            if self.editors.contains_key(&active_path) {
                self.active = Some(active_path);
            }
        }

        Ok(())
    }

    pub fn to_memento(&self) -> WorkspaceMemento {
        let mut open_files = HashMap::new();
        for (p, e) in &self.editors {
            open_files.insert(
                // to_string_lossy(): 以有损方式生成UTF-8文本。
                p.to_string_lossy().into_owned(),
                FileFlags {
                    modified: e.is_modified(),
                    logging: e.logging_enabled(),
                    kind: e.kind(),
                },
            );
        }
        WorkspaceMemento {
            open_files,
            active: self
                .active
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        }
    }

    /// 把命令行参数里的 path 解析成最终要用的绝对/规范路径：
    /// - None      -> base_dir
    /// - 绝对路径   -> 原样返回
    /// - 相对路径   -> base_dir.join(path)
    pub fn resolve_path(&self, arg: Option<&str>) -> PathBuf {
        match arg {
            Some(s) => {
                let p = PathBuf::from(s);
                if p.is_absolute() {
                    p
                } else {
                    self.base_dir.join(p)
                }
            }
            None => self.base_dir.clone(),
        }
    }

    pub fn get_base_dir(&self) -> PathBuf {
        self.base_dir.clone()
    }

    pub fn log_on(&mut self, path: impl AsRef<Path>) -> AppResult<()> {
        let p = path.as_ref();
        let key: PathBuf = p.to_path_buf();

        let ed = self
            .editors
            .get_mut(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such path".into()))?;
        ed.set_logging(true);
        Ok(())
    }

    pub fn log_off(&mut self, path: impl AsRef<Path>) -> AppResult<()> {
        let p = path.as_ref();
        let key: PathBuf = p.to_path_buf();

        let ed = self
            .editors
            .get_mut(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such path".into()))?;
        ed.set_logging(false);
        Ok(())
    }

    pub fn log_show(&self, path: impl AsRef<Path>) -> AppResult<String> {
        let p = path.as_ref();

        // 拿到文件名，决定日志文件名
        let file_name = p
            .file_name()
            .ok_or_else(|| AppError::InvalidArgs(format!(
                "invalid file path for log-show: {}",
                p.display()
            )))?
            .to_string_lossy()
            .into_owned();

        let log_path = self.base_dir.join(format!(".{}.log", file_name));

        if !log_path.exists() {
            return Err(AppError::InvalidArgs(format!(
                "log file not found: {}",
                log_path.display()
            )));
        }

        let content = fs::read_to_string(&log_path).map_err(AppError::Io)?;
        Ok(content)
    }

    // 辅助函数
    fn get_active_editor_mut(&mut self) -> AppResult<&mut EditorInstance> {
        let path = self
            .active
            .clone()
            .ok_or_else(|| AppError::InternalError("no active file.".into()))?;
        
        self.editors
            .get_mut(&path)
            .ok_or_else(|| AppError::InvalidArgs("active editor not found".into()))
    }

    fn is_active_equal_to(&self, borrowed_path: &PathBuf) -> bool {
        self.active
            .as_ref()           // Option<&PathBuf>
            .map(|pb| pb.as_path()) // Option<&Path>
            .map_or(false, |active_path| active_path == borrowed_path.as_path())
    }

    fn write_editor(p: impl AsRef<Path>, is_active: bool, modified: bool) -> String {
        let mut line: String = String::new();
        if is_active {
            line.push_str("* ");
        } else {
            line.push_str("  ");
        }
        let p_str: &str = p.as_ref()
            .to_str()
            .expect("can't parse path");
        line.push_str(p_str);
        if modified {
            line.push_str(" [modified]");
        }
        line
    }

    fn create_editor_for_path(path: &Path, content: &str) -> AppResult<EditorInstance> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("xml") => {
                let mut ed = XmlEditor::new();
                ed.load_from(content)?;
                Ok(EditorInstance::Xml(ed))
            }
            _ => {
                let mut ed = TextEditor::new();
                ed.load_from(content);
                Ok(EditorInstance::Text(ed))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::text_editor::TextEditor;

    /// 每个测试用一个独立的临时目录，并把 workspace.base_dir 指过去
    fn new_temp_workspace() -> (Workspace, tempfile::TempDir) {
        let tmp = tempdir().expect("create tempdir failed");

        let mut ws = Workspace::default();
        // 这里直接改私有字段没问题，因为 tests 是同一模块的子模块
        ws.base_dir = tmp.path().join("work_dir");
        fs::create_dir_all(&ws.base_dir).expect("create work_dir failed");

        (ws, tmp)
    }

    #[test]
    fn resolve_path_uses_base_dir_for_relative_paths() {
        let (ws, _tmp) = new_temp_workspace();
        let base = ws.base_dir.clone();

        // 相对路径：应当挂在 base_dir 下面
        let p1 = ws.resolve_path(Some("foo.txt"));
        assert_eq!(p1, base.join("foo.txt"));

        // 绝对路径：应当原样返回
        let abs = base.join("sub/abs.txt");
        let p2 = ws.resolve_path(Some(abs.to_str().unwrap()));
        assert_eq!(p2, abs);

        // None：约定返回 base_dir 本身
        let p3 = ws.resolve_path(None);
        assert_eq!(p3, base);
    }

    #[test]
    fn save_file_writes_editor_content_to_disk() {
        let (mut ws, _tmp) = new_temp_workspace();

        // 逻辑路径："foo.txt" -> base_dir/foo.txt
        let file_path = ws.resolve_path(Some("foo.txt"));

        // 往 workspace.editors 里塞一个 Editor（不需要对外 API）
        let mut ed = TextEditor::default();
        ed.append_line("hello workspace");
        ws.editors.insert(file_path.clone(), EditorInstance::Text(ed));
        ws.active = Some(file_path.clone());

        // 调用 save_file
        ws.save_file(&file_path).expect("save_file failed");

        // 磁盘上应该出现 base_dir/foo.txt，内容为 "hello workspace"
        let content = fs::read_to_string(&file_path).expect("read saved file failed");
        assert_eq!(content.trim_end(), "hello workspace");
    }

    #[test]
    fn save_all_writes_all_open_editors() {
        let (mut ws, _tmp) = new_temp_workspace();

        let file_a = ws.resolve_path(Some("a.txt"));
        let file_b = ws.resolve_path(Some("subdir/b.txt"));

        // 确保子目录也存在，防止 save_to 里直接 write 报目录不存在
        if let Some(parent) = file_b.parent() {
            fs::create_dir_all(parent).expect("create subdir failed");
        }

        let mut ed_a = TextEditor::default();
        ed_a.append_line("AAAA");
        ws.editors.insert(file_a.clone(), EditorInstance::Text(ed_a));

        let mut ed_b = TextEditor::default();
        ed_b.append_line("BBBB");
        ws.editors.insert(file_b.clone(), EditorInstance::Text(ed_b));

        ws.save_all().expect("save_all failed");

        let content_a = fs::read_to_string(&file_a).expect("read a.txt failed");
        let content_b = fs::read_to_string(&file_b).expect("read b.txt failed");

        assert_eq!(content_a.trim_end(), "AAAA");
        assert_eq!(content_b.trim_end(), "BBBB");
    }

    #[test]
    fn log_show_reads_dot_filename_log_under_base_dir() {
        let (ws, _tmp) = new_temp_workspace();
        let base = ws.base_dir.clone();

        // 源文件路径：假设是 work_dir/main.rs
        let src = base.join("main.rs");
        // Logger 约定的日志路径：work_dir/.main.rs.log
        let log_path = base.join(".main.rs.log");

        fs::write(&log_path, "LOG CONTENT\nLINE 2").expect("write log file failed");

        let content = ws.log_show(&src).expect("log_show failed");
        assert_eq!(content, "LOG CONTENT\nLINE 2");
    }

    #[test]
    fn log_show_returns_error_if_log_not_found() {
        let (ws, _tmp) = new_temp_workspace();
        let base = ws.base_dir.clone();

        let src = base.join("no_log_here.rs");

        let result = ws.log_show(&src);
        assert!(result.is_err(), "expected error when log file is missing");
    }
}
