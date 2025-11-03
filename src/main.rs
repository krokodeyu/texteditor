use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

struct Editor {
    path: Option<PathBuf>,
    buf: Vec<String>,
    modified: bool,
}

impl Editor {
    // Initialize.
    fn new_empty() -> Self {
        Self { path: None, buf: vec![], modified: false}
    }

    fn init_file(&mut self, path: PathBuf, with_log: bool) -> io::Result<()> {
        let head = if with_log { "# log\n" } else { "" };
        fs::write(&path, head)?;
        self.path = Some(path);
        self.buf = if with_log { vec!["# log".into()] } else { vec![] };
        self.modified = false;
        Ok(())
    }

    fn load_file(&mut self, path: PathBuf) -> io::Result<()> {
        let content = fs::read_to_string(&path).unwrap_or_default();
        self.path = Some(path);
        self.buf = if content.is_empty() {
            vec![]
        } else {
            content.split('\n').map(|s| s.to_string()).collect()
        };
        if let Some(last) = self.buf.last() {
            if last.is_empty() { self.buf.pop(); }
        }
        self.modified = false;
        Ok(())
    }

    fn append(&mut self, text: String) {
        self.buf.push(text);
        self.modified = true;
    }

    fn show(&self, start: Option<usize>, end: Option<usize>) {
        let n = self.buf.len();
        if n == 0 {
            println!("<It's an empty file.>");
            return;
        }
        let s = start.unwrap_or(1).max(1).min(n);
        let e = end.unwrap_or(n).max(1).min(n);
        if e < s {
            println!("<Invalid start to end range.>");
            return;
        }

        println!("<file>");
        for i in s..=e {
            println!("{}: {}", i, self.buf[i - 1]);
        }
    }

    fn save(&mut self, file: Option<PathBuf>) -> io::Result<()> {
        if let Some(p) = file {
            self.path = Some(p);
        }

        let p = self.path.clone().ok_or_else(|| io::Error::new(
            io::ErrorKind::Other,
            "No such file. Please 'init <file>' or 'load <file>' first."
        ))?;

        let content = if self.buf.is_empty() {
            String::new()
        } else {
            self.buf.join("\n")
        };
        fs::write(&p, content)?;
        self.modified = false;
        println!("Saved {}", p.display());
        Ok(())
    }
}

fn parse_line(line: &str) -> (String, String) {
    let mut parts = line.trim().splitn(2, ' ');
    let cmd = parts.next().unwrap_or("").to_lowercase();
    let rest = parts.next().unwrap_or("").trim().to_string();
    (cmd, rest)
}

fn parse_range(s: &str) -> (Option<usize>, Option<usize>) {
    if let Some((a, b)) = s.split_once(":") {
        (a.parse().ok(), b.parse().ok())
    } else {
        (None, None)
    }
}

fn main() -> io::Result<()> {
    let mut ed = Editor::new_empty();
    println!("Mini Editor (beginner) — 命令: init/load/append/show/save/exit");

    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        let line = line.trim();
        if line.is_empty() { continue; }

        let (cmd, rest) = parse_line(line);
        match cmd.as_str() {
            "init" => {
                if rest.is_empty() {
                    println!("用法: init <file> [with-log]");
                    continue;
                }
                let mut it = rest.split_whitespace();
                let file = it.next().unwrap();
                let with_log = it.next().map(|s| s.eq_ignore_ascii_case("with-log")).unwrap_or(false);
                match ed.init_file(PathBuf::from(file), with_log) {
                    Ok(_) => println!("已创建并打开 {}", file),
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "load" => {
                if rest.is_empty() { println!("用法: load <file>"); continue; }
                let path = PathBuf::from(&rest);
                match ed.load_file(PathBuf::from(path)) {
                    Ok(_) => println!("已打开 {}", rest),
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "append" => {
                // append 后面的内容整体作为一行
                ed.append(rest);
            }
            "show" => {
                if rest.is_empty() {
                    ed.show(None, None);
                } else {
                    let (s, e) = parse_range(&rest);
                    ed.show(s, e);
                }
            }
            "save" => {
                // save 或 save <file>
                let file_opt = if rest.is_empty() { None } else { Some(PathBuf::from(rest)) };
                if let Err(e) = ed.save(file_opt) {
                    eprintln!("[error] {e}");
                }
            }
            "exit" | "quit" => {
                // 如果被修改但未保存，这里可以提示；为了简洁先直接退出
                println!("再见！");
                break;
            }
            "help" => {
                println!("命令:
  init <file> [with-log]   新建文件（可选首行 # log）
  load <file>              打开文件（不存在则视为空文件）
  append <text>            追加一行
  show [start:end]         查看行范围（缺省为全部）
  save [file]              保存（可指定保存到新文件）
  exit                     退出");
            }
            _ => println!("未知命令：{}（输入 help 查看用法）", cmd),
        }
    }

    Ok(())
}
