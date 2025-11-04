mod editor;
mod workspace;
mod event;
mod logging;
mod persist;

use std::{io::{self, Write}, path::PathBuf};
use workspace::Workspace;
use event::{Bus, Event};
use logging::Logger;
use persist::{load_workspace, save_workspace};

fn parse_line(line: &str) -> (String, String) {
    let mut parts = line.trim().splitn(2, ' ');
    (parts.next().unwrap_or("").to_lowercase(), parts.next().unwrap_or("").trim().to_string())
}
fn parse_range(s: &str) -> (Option<usize>, Option<usize>) {
    if let Some((a,b)) = s.split_once(':') { (a.parse().ok(), b.parse().ok()) } else { (None,None) }
}

fn main() -> io::Result<()> {
    let mut ws = Workspace::new();

    // 事件总线 + 日志订阅
    let mut bus = Bus::new();
    bus.subscribe(Logger::new());
    bus.publish(&Event::SessionStart);

    // 恢复工作区
    if let Some(m) = load_workspace() { ws.from_memento(&m); }

    println!("Mini Editor (Rust, modular) — 命令: init/load/append/show/save/editor-list/exit");
    loop {
        print!("> "); io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        let line = line.trim(); if line.is_empty() { continue; }
        let (cmd, rest) = parse_line(line);

        match cmd.as_str() {
            "init" => {
                if rest.is_empty() { println!("用法: init <file> [with-log]"); continue; }
                let mut it = rest.split_whitespace();
                let file = it.next().unwrap();
                let with_log = it.next().map(|s| s.eq_ignore_ascii_case("with-log")).unwrap_or(false);
                match ws.init_file(file, with_log) {
                    Ok(ed) => {
                        println!("已创建并打开 {}", file);
                        bus.publish(&Event::Command { file: ed.path().cloned(), cmdline: line.to_string(), logging_enabled: ed.logging_enabled() });
                    }
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "load" => {
                if rest.is_empty() { println!("用法: load <file>"); continue; }
                let path = PathBuf::from(&rest);
                match ws.load_file(path) {
                    Ok(ed) => {
                        println!("已打开 {}", rest);
                        bus.publish(&Event::Command { file: ed.path().cloned(), cmdline: line.to_string(), logging_enabled: ed.logging_enabled() });
                    }
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "append" => {
                match ws.active_editor_mut() {
                    Ok(ed) => {
                        ed.append(&rest);
                        bus.publish(&Event::Command { file: ed.path().cloned(), cmdline: line.to_string(), logging_enabled: ed.logging_enabled() });
                    }
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "show" => {
                match ws.active_editor_mut() {
                    Ok(ed) => {
                        if rest.is_empty() {
                            let lines = ed.show(None, None);
                            if lines.is_empty() { println!("<空>"); } else { println!("{}", lines.join("\n")); }
                        } else {
                            let (s,e)=parse_range(&rest);
                            let lines = ed.show(s,e);
                            if lines.is_empty() { println!("<空或范围无效>"); } else { println!("{}", lines.join("\n")); }
                        }
                    }
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "save" => {
                let file_opt = if rest.is_empty() { None } else { Some(PathBuf::from(&rest)) };
                match ws.active_editor_mut() {
                    Ok(ed) => match ed.save(file_opt) {
                        Ok(p) => { println!("已保存 {}", p.display()); }
                        Err(e) => eprintln!("[error] {e}"),
                    },
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
            "editor-list" => {
                for l in ws.list_editors() { println!("{l}"); }
            }
            "exit" => {
                if let Err(e) = save_workspace(&ws.to_memento()) {
                    eprintln!("[warn] 保存工作区失败: {e}");
                }
                println!("再见！");
                break;
            }
            "help" => println!("命令: init/load/append/show/save/editor-list/exit"),
            _ => println!("未知命令：{}（输入 help 查看用法）", cmd),
        }
    }
    Ok(())
}
