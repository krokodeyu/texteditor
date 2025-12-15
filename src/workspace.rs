//! 工作区
//! 管理多文件上下文、状态持久化。
use std::{
    collections::HashMap,
    collections::hash_map::Entry,
    fs, 
    io,
    time::Duration,
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

/// Workspace 可识别的文件类型（命令层用于分流处理）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceFileKind {
    Text,
    Xml,
    _Other(String),
}

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

        match std::fs::metadata(path) {
            Ok(_) => {
                return Err(AppError::InvalidArgs("file already exists on disk!".into()));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // ok
            Err(e) => return Err(AppError::Io(e)),
        }
        
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

    // ===== spell-check 支持：文件枚举/读取/类型判定 =====

    /// 列出当前 workspace 已打开（已加载进 editors）的文件路径。
    /// 返回按字典序排序的路径，便于稳定输出与测试。
    pub fn list_open_files(&self) -> AppResult<Vec<PathBuf>> {
        let mut files: Vec<PathBuf> = self.editors.keys().cloned().collect();
        files.sort();
        Ok(files)
    }

    /// 判断某个已打开文件的类型。
    ///
    /// 说明：这里以 editors 内存中的 editor 类型为准，而不是仅凭后缀名推断。
    pub fn file_kind(&self, p: impl AsRef<Path>) -> AppResult<WorkspaceFileKind> {
        let key: PathBuf = p.as_ref().to_path_buf();
        let ed = self.editors
            .get(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such file!".into()))?;

        Ok(match ed {
            EditorInstance::Text(_) => WorkspaceFileKind::Text,
            EditorInstance::Xml(_) => WorkspaceFileKind::Xml,
        })
    }

    /// 读取某个已打开文本文件的全文（基于 editor 当前内存状态，而非磁盘）。
    /// - 空文件返回空字符串
    /// - 若目标不是文本文件则返回错误
    pub fn read_text_all(&self, p: impl AsRef<Path>) -> AppResult<String> {
        let key: PathBuf = p.as_ref().to_path_buf();
        let ed = self.editors
            .get(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such file!".into()))?
            .as_text()?;

        let n = ed.count_lines();
        if n == 0 {
            return Ok(String::new());
        }
        Ok(ed.show(1, n))
    }

    /// 提取 XML 文件“元素文本内容”，用于拼写检查（不包含标签名、属性名、属性值）。
    ///
    /// 返回 (label, text)：
    /// - label 形如 "title1"、"author2"（同名标签递增编号）
    /// - text 为该元素的“直接文本子节点”拼接结果（trim 后非空）
    ///
    /// 依赖：`roxmltree`
    pub fn extract_xml_element_texts(
        &self,
        p: impl AsRef<Path>,
    ) -> AppResult<Vec<(String, String)>> {
        let key: PathBuf = p.as_ref().to_path_buf();
        let ed = self.editors
            .get(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such file!".into()))?
            .as_xml()?;

        let xml_text = ed.show_all()?;
        let doc = roxmltree::Document::parse(&xml_text)
            .map_err(|e| AppError::InvalidArgs(format!("xml parse failed: {e}")))?;

        let mut out: Vec<(String, String)> = Vec::new();

        for node in doc.descendants().filter(|n| n.is_element()) {
            // Lab2: 每个元素必须有唯一 id
            let id = node.attribute("id")
                .ok_or_else(|| AppError::InvalidArgs("missing id attribute".into()))?
                .to_string();

            // 只取直接文本子节点（避免把子元素文本算到父元素）
            let mut buf = String::new();
            for child in node.children().filter(|c| c.is_text()) {
                if let Some(t) = child.text() {
                    let t = t.trim();
                    if t.is_empty() { continue; }
                    if !buf.is_empty() { buf.push(' '); }
                    buf.push_str(t);
                }
            }

            if !buf.is_empty() {
                out.push((id, buf));
            }
        }

        Ok(out)
    }


    pub fn editor_list(
        &self,
        times: Option<&std::collections::HashMap<PathBuf, Duration>>,
    ) -> AppResult<String> {
        let mut editor_list: String = String::new();
        for (path, editor) in &self.editors {
            let is_active: bool = self.is_active_equal_to(path);
            let modified: bool = editor.is_modified();
            let dur = times.and_then(|m| m.get(path));
            let line = Self::write_editor(path, is_active, modified, dur);
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

    pub fn check_modified(&self) -> bool {
        // for (_, ed) in &self.editors {
        //     if ed.is_modified() {
        //         return true;
        //     }
        // }
        // false
        self.editors.values().any(|ed| ed.is_modified())
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

    fn write_editor(
        p: impl AsRef<Path>,
        is_active: bool,
        modified: bool,
        dur: Option<&Duration>,
    ) -> String {
        let mut line: String = String::new();
        if is_active {
            line.push_str("* ");
        } else {
            line.push_str("  ");
        }
        let p_str: &str = p
            .as_ref()
            .to_str()
            .expect("can't parse path");
        line.push_str(p_str);
        if modified {
            line.push_str(" [modified]");
        }
        if let Some(d) = dur {
            use std::fmt::Write as _;
            let human = Self::format_duration_cn(d);
            let _ = write!(line, " ({})", human);
        }
        line
    }

    fn format_duration_cn(dur: &std::time::Duration) -> String {
        let secs = dur.as_secs();

        // < 1分钟：X秒
        if secs < 60 {
            return format!("{}秒", secs);
        }

        // 1-59分钟：X分钟（忽略秒）
        let mins = secs / 60;
        if mins < 60 {
            return format!("{}分钟", mins);
        }

        // 1-23小时：X小时Y分钟（忽略秒）
        let hours = secs / 3600;
        if hours < 24 {
            let m = (secs % 3600) / 60;
            return format!("{}小时{}分钟", hours, m);
        }

        // ≥ 24小时：X天Y小时（忽略分钟秒）
        let days = hours / 24;
        let h = hours % 24;
        format!("{}天{}小时", days, h)
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

        use crate::error::AppError;
    use crate::commands::xml_command::XmlCommand as XmlCmd;
    use crate::xml_editor::{XmlEditor, DeletedNodeToken};
    use roxmltree::Document;
    use std::collections::HashMap;

    fn write_file(p: &Path, s: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, s).unwrap();
    }

    fn parse_xml(s: &str) -> Document<'_> {
        Document::parse(s).expect("xml parse failed in test")
    }

    // ====== Lab2: init xml ======

    #[test]
    fn init_xml_creates_empty_root_with_id_root_and_xml_decl() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("config.xml"));

        ws.init(EditorKind::Xml, &p, false).unwrap();
        assert_eq!(ws.active_file_path().as_deref(), Some(p.as_path()));

        ws.save_file(&p).unwrap();
        let content = fs::read_to_string(&p).unwrap();

        // XML 声明必须在首行（至少要以 <?xml 开头）
        let first = content.lines().next().unwrap_or("");
        assert!(first.trim_start().starts_with("<?xml"), "first line = `{first}`");

        let doc = parse_xml(&content);
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "root");
        assert_eq!(root.attribute("id"), Some("root"));
        assert_eq!(root.attribute("log"), None);
    }

    #[test]
    fn init_xml_with_log_sets_root_log_true_attribute() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("config_log.xml"));

        ws.init(EditorKind::Xml, &p, true).unwrap();
        ws.save_file(&p).unwrap();
        let content = fs::read_to_string(&p).unwrap();

        let doc = parse_xml(&content);
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "root");
        assert_eq!(root.attribute("id"), Some("root"));
        assert_eq!(root.attribute("log"), Some("true"));
    }

    // ====== Lab2: editor-list duration formatting ======

    #[test]
    fn editor_list_prints_human_duration_and_supports_days_format() {
        let (mut ws, _tmp) = new_temp_workspace();
        let f1 = ws.resolve_path(Some("file1.txt"));
        let f2 = ws.resolve_path(Some("file2.xml"));
        let f3 = ws.resolve_path(Some("file3.txt"));

        ws.init(EditorKind::Text, &f1, false).unwrap();
        ws.init(EditorKind::Xml, &f2, false).unwrap();
        ws.init(EditorKind::Text, &f3, false).unwrap();
        ws.edit(&f1).unwrap(); // 让 f1 成为 active

        let mut times: HashMap<PathBuf, Duration> = HashMap::new();
        times.insert(f1.clone(), Duration::from_secs(2 * 3600 + 15 * 60)); // 2小时15分钟
        times.insert(f2.clone(), Duration::from_secs(45));                // 45秒
        times.insert(f3.clone(), Duration::from_secs(27 * 3600));         // 1天3小时（>=24小时）

        let out = ws.editor_list(Some(&times)).unwrap();

        // 不依赖 HashMap 输出顺序：逐行找关键片段
        let lines: Vec<&str> = out.lines().collect();

        let l1 = lines.iter().find(|l| l.contains("file1.txt")).unwrap();
        assert!(l1.starts_with("* "), "active mark missing: {l1}");
        assert!(l1.contains("(2小时15分钟)"), "duration missing/wrong: {l1}");

        let l2 = lines.iter().find(|l| l.contains("file2.xml")).unwrap();
        assert!(l2.contains("(45秒)"), "duration missing/wrong: {l2}");

        let l3 = lines.iter().find(|l| l.contains("file3.txt")).unwrap();
        assert!(l3.contains("(1天3小时)"), ">=24h should be days+hours: {l3}");
    }

    // ====== Lab2: spell-check data extraction (workspace side) ======

    #[test]
    fn extract_xml_element_texts_should_use_element_id_and_only_text_nodes() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("spell.xml"));

        // title1/author2 等按“元素 id”输出（而不是 tag 计数）
        // 且只检查元素文本，不检查标签/属性（这里不测试算法，只测试抽取范围）
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<bookstore id="root">
  <book id="book1" category="COOKING">
    <title id="title1" lang="en">Itallian</title>
    <author id="author2">Rowlling</author>
    <meta id="meta1">
      <inner id="inner1">OK</inner>
    </meta>
  </book>
</bookstore>
"#;

        write_file(&p, xml);
        ws.load(&p).unwrap();

        let mut got = ws.extract_xml_element_texts(&p).unwrap();
        got.sort_by(|a, b| a.0.cmp(&b.0));

        // 期望：按元素 id 返回
        // 期望：包含所有“有文本的元素”（title1/author2/inner1），不包含 meta1/book1/bookstore 这种“只有子元素”的节点
        assert_eq!(
            got,
            vec![
                ("author2".to_string(), "Rowlling".to_string()),
                ("inner1".to_string(), "OK".to_string()),
                ("title1".to_string(), "Itallian".to_string()),
            ]
        );
    }

    // ====== Lab2: XML editing commands semantics (via workspace.exec_xml) ======

    struct CmdInsertBefore {
        tag: String,
        new_id: String,
        target_id: String,
        text: Option<String>,
    }
    impl XmlCmd for CmdInsertBefore {
        fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            ed.insert_before(&self.tag, &self.new_id, &self.target_id, self.text.as_ref())
        }
        fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            let _ = ed.delete_node(&self.new_id);
            Ok(())
        }
    }

    struct CmdAppendChild {
        tag: String,
        new_id: String,
        parent_id: String,
        text: Option<String>,
    }
    impl XmlCmd for CmdAppendChild {
        fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            ed.append_child(&self.tag, &self.new_id, &self.parent_id, self.text.as_ref())
        }
        fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            let _ = ed.delete_node(&self.new_id)?;
            Ok(())
        }
    }

    struct CmdEditId {
        old_id: String,
        new_id: String,
    }
    impl XmlCmd for CmdEditId {
        fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            ed.change_attr_id(&self.old_id, &self.new_id)
        }
        fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            ed.change_attr_id(&self.new_id, &self.old_id)
        }
    }

    struct CmdEditText {
        id: String,
        new_text: String,
        old: Option<String>,
    }
    impl XmlCmd for CmdEditText {
        fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            self.old = ed.change_text(&self.id, &self.new_text)?;
            Ok(())
        }
        fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            match &self.old {
                Some(t) => { let _ = ed.change_text(&self.id, t)?; }
                None => { let _ = ed.remove_text(&self.id)?; }
            }
            Ok(())
        }
    }

    struct CmdDeleteElement {
        id: String,
        token: Option<DeletedNodeToken>,
    }
    impl XmlCmd for CmdDeleteElement {
        fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            self.token = Some(ed.delete_node(&self.id)?);
            Ok(())
        }
        fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
            let token = self.token.take().ok_or_else(|| AppError::InternalError("missing token".into()))?;
            ed.restore_node(token)
        }
    }

    #[test]
    fn xml_commands_happy_path_and_undo_delete() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("books.xml"));

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<bookstore id="root">
  <book id="book1">
    <title id="title1">First Book</title>
  </book>
</bookstore>
"#;
        write_file(&p, xml);
        ws.load(&p).unwrap();

        // insert-before: 在 book1 前插入 newBook
        ws.exec_xml(Box::new(CmdInsertBefore{
            tag: "book".into(),
            new_id: "newBook".into(),
            target_id: "book1".into(),
            text: Some("".into()),
        })).unwrap();

        // append-child: 给 book1 追加 price4
        ws.exec_xml(Box::new(CmdAppendChild{
            tag: "price".into(),
            new_id: "price4".into(),
            parent_id: "book1".into(),
            text: Some("29.99".into()),
        })).unwrap();

        // edit-id: book1 -> book001
        ws.exec_xml(Box::new(CmdEditId{
            old_id: "book1".into(),
            new_id: "book001".into(),
        })).unwrap();

        // edit-text: title1 -> New Book Title
        ws.exec_xml(Box::new(CmdEditText{
            id: "title1".into(),
            new_text: "New Book Title".into(),
            old: None,
        })).unwrap();

        // delete-element: 删除 book001
        ws.exec_xml(Box::new(CmdDeleteElement{
            id: "book001".into(),
            token: None,
        })).unwrap();

        // undo: 恢复 book001
        ws.undo().unwrap();

        // 校验最终 XML 结构（不依赖具体序列化空白）
        let xml_now = ws.show_xml_tree(&p).unwrap(); // 这里按“存储内容”检查，要求能被 XML parser 解析
        let doc = parse_xml(&xml_now);
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "bookstore");

        let books: Vec<&str> = root
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "book")
            .filter_map(|n| n.attribute("id"))
            .collect();

        // 顺序：newBook 在 book001 前
        assert_eq!(books, vec!["newBook", "book001"]);

        // book001 里要有 title1 文本与 price4 文本
        let book001 = root
            .children()
            .find(|n| n.is_element() && n.attribute("id") == Some("book001"))
            .unwrap();

        let title = book001
            .children()
            .find(|n| n.is_element() && n.attribute("id") == Some("title1"))
            .unwrap();
        assert_eq!(title.text(), Some("New Book Title"));

        let price = book001
            .children()
            .find(|n| n.is_element() && n.attribute("id") == Some("price4"))
            .unwrap();
        assert_eq!(price.text(), Some("29.99"));
    }

    #[test]
    fn xml_mixed_content_rejected_append_child_when_parent_has_text() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("mixed.xml"));

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<root id="root">
  <p id="p1">hello</p>
</root>
"#;
        write_file(&p, xml);
        ws.load(&p).unwrap();

        let r = ws.exec_xml(Box::new(CmdAppendChild{
            tag: "x".into(),
            new_id: "x1".into(),
            parent_id: "p1".into(),
            text: Some("child".into()),
        }));

        // 约束：不支持混合内容（父已有文本时不能再加子元素）
        assert!(r.is_err());
    }

    #[test]
    fn xml_edit_text_rejected_when_element_has_children() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("has_children.xml"));

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<root id="root">
  <book id="book1">
    <title id="title1">T</title>
  </book>
</root>
"#;
        write_file(&p, xml);
        ws.load(&p).unwrap();

        let r = ws.exec_xml(Box::new(CmdEditText{
            id: "book1".into(),
            new_text: "SHOULD_FAIL".into(),
            old: None,
        }));

        // 约束：元素有子元素时，不允许 edit-text（避免混合内容）
        assert!(r.is_err());
    }

    #[test]
    fn xml_tree_output_should_contain_structure_and_text() {
        let (mut ws, _tmp) = new_temp_workspace();
        let p = ws.resolve_path(Some("tree.xml"));

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<bookstore id="root">
  <book id="book1" category="COOKING">
    <title id="title1" lang="en">Everyday Italian</title>
  </book>
</bookstore>
"#;
        write_file(&p, xml);
        ws.load(&p).unwrap();

        let out = ws.show_xml_tree(&p).unwrap();

        // 输出格式允许树形字符或缩进，但至少要展示层级、属性、文本内容
        assert!(out.contains("bookstore") && out.contains("id=\"root\""), "missing root info: {out}");
        assert!(out.contains("Everyday Italian"), "missing text node: {out}");

        let looks_like_tree = out.contains("└──") || out.contains("├──") || out.contains("\n  ");
        assert!(looks_like_tree, "output doesn't look like tree/indent format: {out}");
    }
}
