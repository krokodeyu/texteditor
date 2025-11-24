"""
tests/random_cli_fuzz.py

随机用户层测试（Fuzzing 风格）：

- 随机生成命令序列（load / append / init / edit / save / undo / redo / dir-tree / log-* / 随机垃圾命令）；
- 每个会话在一个独立目录下运行被测二进制；
- 记录每个会话的：
    * 随机命令列表
    * stdout
    * stderr
    * 退出码
  到项目根目录的 fuzz_runs/session_<i>.log 中，方便人工分析。

使用方法：
    1. 在项目根目录执行：cargo build
    2. 运行本脚本：python tests/random_cli_fuzz.py
"""

import subprocess
import random
import string
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Optional


# =======================
# 配置
# =======================

# 你的二进制名：target/debug/<这个名字>
EDITOR_BIN_NAME = "texteditor.exe"  # TODO: 改成你自己的二进制文件名

# 随机测试参数
SEED = 114514                 # 随机种子
NUM_SESSIONS = 4             # 跑多少个独立会话
COMMANDS_PER_SESSION = 50     # 每个会话随机发多少条命令（不含最后的 exit）

# 生成的测试日志目录（相对于项目根目录）
FUZZ_RUNS_DIR_NAME = "fuzz_runs"


# =======================
# 辅助结构 & 随机生成逻辑
# =======================

@dataclass
class FuzzState:
    """维护当前会话的一些状态，方便生成“有点合理”的随机命令。"""
    known_files: List[str] = field(default_factory=list)  # 我们曾经提到/打开过的文件名
    cwd: Path = Path(".")                                 # 当前会话的工作目录（根目录下的某个 run_xx）
    base_dir_name: str = "work_dir"                       # Workspace base_dir 名字（你的 Rust 里就是这个）

    def random_known_file(self) -> Optional[str]:
        """随机选一个已经见过的文件名；如果还没有，就返回 None。"""
        if not self.known_files:
            return None
        return random.choice(self.known_files)

    def remember_file(self, name: str) -> None:
        if name not in self.known_files:
            self.known_files.append(name)


def random_identifier(min_len=1, max_len=8) -> str:
    """随机生成一个“看起来像文件名/命令片段”的字符串。"""
    length = random.randint(min_len, max_len)
    chars = string.ascii_lowercase + string.digits
    return "".join(random.choice(chars) for _ in range(length))


def random_filename() -> str:
    """随机生成一个文件名，可能带一层子目录。"""
    # 例如：visitor.txt / logs/a.rs / a/b/c.txt 也可以扩展
    if random.random() < 0.3:
        sub = random_identifier()
        name = random_identifier()
        ext = random.choice([".txt", ".rs", ".log", ".md", ".sc"])
        return f"{sub}/{name}{ext}"
    else:
        name = random_identifier()
        ext = random.choice([".txt", ".rs", ".md", ".sc", ".cfg", ""])
        return f"{name}{ext}"


def random_text(max_words=5) -> str:
    """随机生成一行文本，用于 append / insert 等。"""
    words = []
    for _ in range(random.randint(1, max_words)):
        words.append(random_identifier(1, 6))
    return " ".join(words)


def gen_random_command(state: FuzzState) -> str:
    """
    生成一条随机命令（单行字符串），命令名从预设列表中随机选择，
    参数则根据类型随机（部分使用 known_files，部分完全新生成）。
    """
    # 包括一些“有意义”的命令和完全随机的垃圾命令
    command_kinds = [
        "load",
        "append",
        "init",
        "editor-list",
        "show",
        "edit",
        "save",
        "undo",
        "redo",
        "dir-tree",
        "log-on",
        "log-off",
        "log-show",
        "garbage",  # 完全无意义命令，测试错误处理
    ]

    kind = random.choice(command_kinds)

    # 针对不同命令类型，构造参数
    if kind == "load":
        # 部分时候复用已知文件，部分时候生成新文件
        if random.random() < 0.5 and state.known_files:
            fname = state.random_known_file()
        else:
            fname = random_filename()
        state.remember_file(fname)
        return f"load {fname}"

    elif kind == "append":
        text = random_text()
        # 注意加引号，尽量保持和你现有命令解析一致
        return f'append "{text}"'

    elif kind == "init":
        if random.random() < 0.5 and state.known_files:
            fname = state.random_known_file()
        else:
            fname = random_filename()
        state.remember_file(fname)
        # 50% 概率带 with-log
        if random.random() < 0.5:
            return f"init {fname} with-log"
        else:
            return f"init {fname}"

    elif kind == "editor-list":
        return "editor-list"

    elif kind == "show":
        # 有概率带上行号范围
        if random.random() < 0.3:
            start = random.randint(1, 5)
            end = start + random.randint(0, 5)
            return f"show {start} {end}"
        else:
            return "show"

    elif kind == "edit":
        fname = state.random_known_file()
        if fname is None:
            # 没有已知文件就随便找个名字
            fname = random_filename()
            state.remember_file(fname)
        return f"edit {fname}"

    elif kind == "save":
        # save / save all / save 某个文件
        r = random.random()
        if r < 0.3:
            return "save"
        elif r < 0.6:
            return "save all"
        else:
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename()
                state.remember_file(fname)
            return f"save {fname}"

    elif kind == "undo":
        return "undo"

    elif kind == "redo":
        return "redo"

    elif kind == "dir-tree":
        # 50% 无参数（默认 base_dir），50% 随机路径
        if random.random() < 0.5:
            return "dir-tree"
        else:
            path = random_filename()
            return f"dir-tree {path}"

    elif kind == "log-on":
        # 无参时默认当前文件，有参时随机文件
        if random.random() < 0.5:
            return "log-on"
        else:
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename()
                state.remember_file(fname)
            return f"log-on {fname}"

    elif kind == "log-off":
        if random.random() < 0.5:
            return "log-off"
        else:
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename()
                state.remember_file(fname)
            return f"log-off {fname}"

    elif kind == "log-show":
        if random.random() < 0.5:
            return "log-show"
        else:
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename()
                state.remember_file(fname)
            return f"log-show {fname}"

    else:  # "garbage"
        # 随机拼一串乱七八糟的 token，测试错误处理
        token_count = random.randint(1, 4)
        tokens = [random_identifier() for _ in range(token_count)]
        return " ".join(tokens)


# =======================
# 运行 & 记录
# =======================

def run_editor_session(session_id: int, project_root: Path) -> None:
    """
    运行一次随机会话：
    - 在 fuzz_runs/run_<id> 目录下运行编辑器；
    - 生成 COMMANDS_PER_SESSION 条随机命令 + exit；
    - 调用二进制；
    - 将命令 + stdout + stderr + 退出码写到 fuzz_runs/session_<id>.log。
    """
    random.seed(SEED + session_id)  # 每个会话稍微偏移一下种子

    editor_bin = project_root / "target" / "debug" / EDITOR_BIN_NAME
    if not editor_bin.exists():
        raise RuntimeError(f"可执行文件不存在: {editor_bin}\n请先在项目根目录执行: cargo build")

    fuzz_root = project_root / FUZZ_RUNS_DIR_NAME
    fuzz_root.mkdir(exist_ok=True)

    # 每个会话一个独立的工作目录
    cwd = fuzz_root / f"run_{session_id}"
    cwd.mkdir(exist_ok=True)

    state = FuzzState(
        known_files=[],
        cwd=cwd,
        base_dir_name="work_dir",
    )

    # 生成随机命令序列
    commands: List[str] = []
    for _ in range(COMMANDS_PER_SESSION):
        cmd = gen_random_command(state)
        commands.append(cmd)

    # 确保最后有 exit
    if random.random() < 0.5:
        commands.append("save")
    if not commands or commands[-1].strip() != "exit":
        commands.append("exit")

    input_data = "\n".join(commands) + "\n"

    result = subprocess.run(
        [str(editor_bin)],
        input=input_data.encode("utf-8"),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(cwd),
        timeout=10.0,
    )

    stdout = result.stdout.decode("utf-8", errors="ignore")
    stderr = result.stderr.decode("utf-8", errors="ignore")
    code = result.returncode

    # 把本次会话的所有信息写到 log 文件
    log_path = fuzz_root / f"session_{session_id}.log"
    with log_path.open("w", encoding="utf-8") as f:
        f.write(f"# Session {session_id}\n")
        f.write(f"# Work dir: {cwd}\n")
        f.write(f"# Exit code: {code}\n")
        f.write("\n## Commands\n")
        for c in commands:
            f.write(f"> {c}\n")
        f.write("\n## STDOUT\n")
        f.write(stdout)
        f.write("\n\n## STDERR\n")
        f.write(stderr)

    print(f"[fuzz] Session {session_id} done, exit={code}, log={log_path}")

    # 简单的自动检查：不崩溃、不 panic
    if code != 0:
        print(f"[warn] Session {session_id} exit code != 0")
    if "panic" in stderr.lower():
        print(f"[warn] Session {session_id} stderr contains 'panic'")


def main():
    project_root = Path(__file__).parents[1]
    print(f"[fuzz] Project root = {project_root}")

    for sid in range(NUM_SESSIONS):
        run_editor_session(sid, project_root)

    print("\n[fuzz] All sessions finished.")
    print(f"[fuzz] Logs are under: {project_root / FUZZ_RUNS_DIR_NAME}")
    print("[fuzz] 请人工抽查若干 session_*.log，确认行为是否合理（无明显逻辑错误）。")


if __name__ == "__main__":
    main()
