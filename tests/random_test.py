"""
tests/random_cli_fuzz.py

随机用户层测试（Fuzzing 风格，Lab2 适配版）：

- 随机生成命令序列（包含 Lab1 命令 + Lab2 新增/修改命令）
- 会话内逐条写入 stdin，并在两条命令之间随机 sleep，以触发“编辑时长统计”
- 每个会话在独立目录下运行被测二进制
- 记录命令、stdout、stderr、退出码到 fuzz_runs/session_<i>.log 方便人工分析

使用方法：
    1) cargo build
    2) python tests/random_test.py
"""

import subprocess
import random
import string
import time
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Set


# =======================
# 配置
# =======================

EDITOR_BIN_NAME = "texteditor.exe"  # 改成你的 target/debug 下的二进制名

SEED = 114514
NUM_SESSIONS = 4
COMMANDS_PER_SESSION = 70

FUZZ_RUNS_DIR_NAME = "fuzz_runs"

# 为了触发计时统计：两条命令之间随机等待（秒）
SLEEP_MIN = 0.00
SLEEP_MAX = 0.25


# =======================
# 会话状态
# =======================

@dataclass
class FuzzState:
    known_files: List[str] = field(default_factory=list)
    xml_files: List[str] = field(default_factory=list)
    text_files: List[str] = field(default_factory=list)

    # 维护“可能存在的 xml 元素 id”，提高 XML 命令成功概率
    xml_ids: Dict[str, Set[str]] = field(default_factory=dict)

    cwd: Path = Path(".")
    base_dir_name: str = "work_dir"

    def remember_file(self, name: str) -> None:
        if name not in self.known_files:
            self.known_files.append(name)

        if name.endswith(".xml"):
            if name not in self.xml_files:
                self.xml_files.append(name)
            self.xml_ids.setdefault(name, set()).add("root")  # init xml 至少有 root
        else:
            if name not in self.text_files:
                self.text_files.append(name)

    def random_known_file(self) -> Optional[str]:
        return random.choice(self.known_files) if self.known_files else None

    def random_xml_file(self) -> Optional[str]:
        return random.choice(self.xml_files) if self.xml_files else None

    def random_text_file(self) -> Optional[str]:
        return random.choice(self.text_files) if self.text_files else None

    def fresh_xml_id(self, file: str, min_len=3, max_len=10) -> str:
        used = self.xml_ids.setdefault(file, set())
        while True:
            s = random_identifier(min_len, max_len)
            if s != "root" and s not in used:
                used.add(s)
                return s

    def pick_existing_xml_id(self, file: str, allow_root: bool = True) -> Optional[str]:
        ids = list(self.xml_ids.get(file, set()))
        if not ids:
            return None
        if not allow_root:
            ids = [x for x in ids if x != "root"]
            if not ids:
                return None
        return random.choice(ids)

    def maybe_remove_xml_id(self, file: str, elem_id: str) -> None:
        if file in self.xml_ids and elem_id in self.xml_ids[file]:
            self.xml_ids[file].remove(elem_id)


# =======================
# 随机生成辅助
# =======================

def random_identifier(min_len=1, max_len=8) -> str:
    length = random.randint(min_len, max_len)
    chars = string.ascii_lowercase + string.digits
    return "".join(random.choice(chars) for _ in range(length))


def random_filename(ext_pool=None) -> str:
    if ext_pool is None:
        ext_pool = [".txt", ".xml", ".rs", ".md", ".log", ".cfg", ""]
    ext = random.choice(ext_pool)

    if random.random() < 0.30:
        sub = random_identifier()
        name = random_identifier()
        return f"{sub}/{name}{ext}"
    else:
        name = random_identifier()
        return f"{name}{ext}"


def random_text(max_words=6) -> str:
    words = [random_identifier(1, 6) for _ in range(random.randint(1, max_words))]
    return " ".join(words)


def random_tag_name() -> str:
    # 简单 tagName：字母开头
    return "t" + random_identifier(2, 8)


def gen_random_command(state: FuzzState) -> str:
    """
    生成一条随机命令。设计目标：
    - 覆盖 Lab2 的 init / editor-list / spell-check / XML 6 命令
    - 也保留 Lab1 常见命令以做回归
    - 保持“有点合理”，但允许大量无效输入以覆盖错误处理
    """
    command_kinds = [
        # ===== Lab1 / 基础 =====
        "load",
        "append",
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
        # ===== Lab2 修改/新增 =====
        "init",            # init text|xml <file> [with-log]
        "xml-tree",        # xml-tree [file]
        "insert-before",   # insert-before <tag> <newId> <targetId> ["text"]
        "append-child",    # append-child <tag> <newId> <parentId> ["text"]
        "edit-id",         # edit-id <oldId> <newId>
        "edit-text",       # edit-text <elementId> "text"
        "delete-element",  # delete-element <elementId>
        "spell-check",     # spell-check [file] + 你实现的 --all
        # ===== 垃圾输入 =====
        "garbage",
    ]

    kind = random.choice(command_kinds)

    # -----------------------
    # init（Lab2：必须带 text/xml）
    # -----------------------
    if kind == "init":
        file_kind = "xml" if random.random() < 0.35 else "text"
        if file_kind == "xml":
            fname = random_filename(ext_pool=[".xml"])
        else:
            fname = random_filename(ext_pool=[".txt", ".md", ".log", ""])
        state.remember_file(fname)

        with_log = (random.random() < 0.5)
        return f"init {file_kind} {fname}" + (" with-log" if with_log else "")

    # -----------------------
    # load
    # -----------------------
    if kind == "load":
        if random.random() < 0.6 and state.known_files:
            fname = state.random_known_file()
        else:
            fname = random_filename(ext_pool=[".txt", ".xml", ".md", ".log", ""])
        state.remember_file(fname)
        return f"load {fname}"

    # -----------------------
    # edit（切换活动文件，触发计时切换）
    # -----------------------
    if kind == "edit":
        fname = state.random_known_file()
        if fname is None:
            fname = random_filename(ext_pool=[".txt", ".xml"])
            state.remember_file(fname)
        return f"edit {fname}"

    # -----------------------
    # append（仅文本编辑器常用；XML 会报错也没关系）
    # -----------------------
    if kind == "append":
        text = random_text()
        return f'append "{text}"'

    # -----------------------
    # show（Lab1 语法不确定：兼容生成几种常见形式）
    # -----------------------
    if kind == "show":
        r = random.random()
        if r < 0.40:
            return "show"
        elif r < 0.70:
            # show N
            start = random.randint(1, 8)
            return f"show {start}"
        else:
            # show N M
            start = random.randint(1, 6)
            end = start + random.randint(0, 6)
            return f"show {start} {end}"

    # -----------------------
    # save
    # -----------------------
    if kind == "save":
        r = random.random()
        if r < 0.40:
            return "save"
        elif r < 0.70:
            return "save all"
        else:
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename(ext_pool=[".txt", ".xml"])
                state.remember_file(fname)
            return f"save {fname}"

    # -----------------------
    # undo/redo/editor-list
    # -----------------------
    if kind in ("undo", "redo", "editor-list"):
        return kind

    # -----------------------
    # dir-tree
    # -----------------------
    if kind == "dir-tree":
        if random.random() < 0.5:
            return "dir-tree"
        else:
            p = random_filename()
            return f"dir-tree {p}"

    # -----------------------
    # log-on/off/show
    # -----------------------
    if kind in ("log-on", "log-off", "log-show"):
        if random.random() < 0.5:
            return kind
        fname = state.random_known_file()
        if fname is None:
            fname = random_filename(ext_pool=[".txt", ".xml"])
            state.remember_file(fname)
        return f"{kind} {fname}"

    # -----------------------
    # xml-tree
    # -----------------------
    if kind == "xml-tree":
        if random.random() < 0.5:
            return "xml-tree"
        fname = state.random_xml_file()
        if fname is None:
            # 还没有 xml 文件：先 init 一个，提升后续 XML 命令覆盖率
            fname = random_filename(ext_pool=[".xml"])
            state.remember_file(fname)
            return f"init xml {fname}"
        return f"xml-tree {fname}"

    # -----------------------
    # XML 编辑命令（6 个）
    # -----------------------
    if kind in ("append-child", "insert-before", "edit-id", "edit-text", "delete-element"):
        xmlf = state.random_xml_file()
        if xmlf is None:
            xmlf = random_filename(ext_pool=[".xml"])
            state.remember_file(xmlf)
            return f"init xml {xmlf}"

        # 注意：这些命令默认作用于“当前活动文件”，我们这里不强求先 edit，
        # 让错误处理也得到覆盖。

        if kind == "append-child":
            tag = random_tag_name()
            new_id = state.fresh_xml_id(xmlf)
            parent_id = state.pick_existing_xml_id(xmlf, allow_root=True) or "root"
            text = "" if random.random() < 0.5 else random_text(4)
            if random.random() < 0.5:
                return f'append-child {tag} {new_id} {parent_id} "{text}"'
            else:
                return f"append-child {tag} {new_id} {parent_id}"

        if kind == "insert-before":
            tag = random_tag_name()
            new_id = state.fresh_xml_id(xmlf)
            # insert-before 不能 target root（通常会报“不能在根前插入”），所以尽量挑非 root
            target_id = state.pick_existing_xml_id(xmlf, allow_root=False)
            if target_id is None:
                # 没有可用 target，就先 append-child 一个，制造 target
                tag2 = random_tag_name()
                nid2 = state.fresh_xml_id(xmlf)
                return f"append-child {tag2} {nid2} root"
            text = "" if random.random() < 0.5 else random_text(4)
            if random.random() < 0.5:
                return f'insert-before {tag} {new_id} {target_id} "{text}"'
            else:
                return f"insert-before {tag} {new_id} {target_id}"

        if kind == "edit-id":
            old_id = state.pick_existing_xml_id(xmlf, allow_root=False)
            if old_id is None:
                return f"append-child {random_tag_name()} {state.fresh_xml_id(xmlf)} root"
            new_id = state.fresh_xml_id(xmlf)
            # 先把 old_id 也留下（不确定命令是否成功），但尽量同步一下状态
            state.maybe_remove_xml_id(xmlf, old_id)
            state.xml_ids.setdefault(xmlf, set()).add(new_id)
            return f"edit-id {old_id} {new_id}"

        if kind == "edit-text":
            elem_id = state.pick_existing_xml_id(xmlf, allow_root=True) or "root"
            text = random_text(6)
            return f'edit-text {elem_id} "{text}"'

        if kind == "delete-element":
            elem_id = state.pick_existing_xml_id(xmlf, allow_root=False)
            if elem_id is None:
                return f"append-child {random_tag_name()} {state.fresh_xml_id(xmlf)} root"
            state.maybe_remove_xml_id(xmlf, elem_id)
            return f"delete-element {elem_id}"

    # -----------------------
    # spell-check（支持你实现的 --all；规范里是 spell-check [file]）
    # -----------------------
    if kind == "spell-check":
        r = random.random()
        if r < 0.40:
            return "spell-check"   # 当前活动文件
        elif r < 0.70:
            # 指定文件
            fname = state.random_known_file()
            if fname is None:
                fname = random_filename(ext_pool=[".txt", ".xml"])
                state.remember_file(fname)
            return f"spell-check {fname}"
        else:
            return "spell-check --all"  # 你扩展的能力（若未实现也能测错误处理）

    # -----------------------
    # garbage
    # -----------------------
    token_count = random.randint(1, 5)
    tokens = [random_identifier() for _ in range(token_count)]
    return " ".join(tokens)


# =======================
# 运行 & 记录
# =======================

def run_editor_session(session_id: int, project_root: Path) -> None:
    random.seed(SEED + session_id)

    editor_bin = project_root / "target" / "debug" / EDITOR_BIN_NAME
    if not editor_bin.exists():
        raise RuntimeError(
            f"可执行文件不存在: {editor_bin}\n"
            f"请先在项目根目录执行: cargo build\n"
            f"并检查 EDITOR_BIN_NAME 是否正确"
        )

    fuzz_root = project_root / FUZZ_RUNS_DIR_NAME
    fuzz_root.mkdir(exist_ok=True)

    cwd = fuzz_root / f"run_{session_id}"
    cwd.mkdir(exist_ok=True)

    state = FuzzState(cwd=cwd, base_dir_name="work_dir")

    commands: List[str] = []
    for _ in range(COMMANDS_PER_SESSION):
        commands.append(gen_random_command(state))

    # 末尾尽量做一次 editor-list 观察计时输出
    commands.append("editor-list")

    # 结束前随机 save 一下
    if random.random() < 0.6:
        commands.append("save")

    # 统一以 exit 收尾
    commands.append("exit")

    # ===== 用 Popen 逐条喂命令，加入 sleep，以触发统计模块 =====
    proc = subprocess.Popen(
        [str(editor_bin)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(cwd),
    )

    assert proc.stdin is not None
    for c in commands:
        proc.stdin.write((c + "\n").encode("utf-8"))
        proc.stdin.flush()
        time.sleep(random.uniform(SLEEP_MIN, SLEEP_MAX))

    proc.stdin.close()

    try:
        stdout_b, stderr_b = proc.communicate(timeout=15.0)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout_b, stderr_b = proc.communicate()

    stdout = stdout_b.decode("utf-8", errors="ignore")
    stderr = stderr_b.decode("utf-8", errors="ignore")
    code = proc.returncode

    log_path = fuzz_root / f"session_{session_id}.log"
    with log_path.open("w", encoding="utf-8") as f:
        f.write(f"# Session {session_id}\n")
        f.write(f"# Work dir: {cwd}\n")
        f.write(f"# Exit code: {code}\n")
        f.write(f"# Sleep range: [{SLEEP_MIN}, {SLEEP_MAX}] seconds\n")
        f.write("\n## Commands\n")
        for c in commands:
            f.write(f"> {c}\n")
        f.write("\n## STDOUT\n")
        f.write(stdout)
        f.write("\n\n## STDERR\n")
        f.write(stderr)

    print(f"[fuzz] Session {session_id} done, exit={code}, log={log_path}")

    # 轻量自动检查
    if code not in (0, None):
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
    print("[fuzz] 建议人工抽查：spell-check 输出格式、xml-tree 树形、editor-list 计时是否随 sleep 变化。")


if __name__ == "__main__":
    main()
