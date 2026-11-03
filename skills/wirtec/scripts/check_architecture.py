#!/usr/bin/env python3
"""检查 C 代码的直接越层依赖、LVGL 泄漏和宏作用域。"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

LAYERS = {"app", "business", "hal", "drivers", "pkg"}
ALLOWED = {
    "app": {"app", "business", "pkg"},
    "business": {"business", "hal", "pkg"},
    "hal": {"hal", "drivers", "pkg"},
    "drivers": {"drivers", "pkg"},
    "pkg": {"pkg"},
}
SOURCE_SUFFIXES = {".c", ".h", ".cc", ".cpp", ".cxx", ".hpp", ".hxx"}
HEADER_SUFFIXES = {".h", ".hpp", ".hxx"}
LOGGER_MACRO_HEADER = Path("src/pkg/logger/log.h")
INCLUDE_RE = re.compile(r'^\s*#\s*include\s*[<"]([^>"]+)[>"]', re.MULTILINE)
LAYER_INCLUDE_RE = re.compile(r"(?:^|/)(?:src/)?(app|business|hal|drivers|pkg)(?:/|$)")
MACRO_DEFINE_RE = re.compile(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)
DIRECTIVE_RE = re.compile(
    r"^\s*#\s*(ifndef|define)\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE
)
LVGL_INCLUDE_RE = re.compile(r"(?:^|/)(?:lvgl|lvgl\.h)(?:/|$)", re.IGNORECASE)
SRC_INCLUDE_RE = re.compile(r"(?:^|/)src/include(?:/|$)")
LVGL_TYPE_RE = re.compile(
    r"\b(?:lv_(?:obj|event|style|color|font|img|image|disp|display|indev|group|"
    r"timer|theme|point|area|draw|anim|canvas|label|btn|button|screen|flex|grid|"
    r"layout|fs|mem|result|coord|opacity|dir|align|state|part)(?:_[A-Za-z0-9]+)*_t)\b"
)


def layer_of(path: Path, root: Path) -> str | None:
    try:
        parts = path.relative_to(root).parts
    except ValueError:
        return None
    if len(parts) >= 2 and parts[0] == "src" and parts[1] in LAYERS:
        return parts[1]
    return None


def iter_files(root: Path, raw_paths: list[str]) -> list[Path]:
    seeds = (
        [Path(value) for value in raw_paths]
        if raw_paths
        else [root / "src" / name for name in sorted(LAYERS)]
    )
    found: set[Path] = set()
    for seed in seeds:
        path = seed.absolute() if seed.is_absolute() else root / seed
        if path.is_file() and path.suffix.lower() in SOURCE_SUFFIXES:
            found.add(path)
        elif path.is_dir():
            found.update(
                p
                for p in path.rglob("*")
                if p.is_file() and p.suffix.lower() in SOURCE_SUFFIXES
            )
    return sorted(found)


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def strip_comments_and_literals(text: str) -> str:
    """保留换行和字符位置，用空格隐藏注释及字符串中的标识符。"""

    pattern = re.compile(
        r"//[^\n]*|/\*.*?\*/|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'",
        re.DOTALL,
    )

    def blank(match: re.Match[str]) -> str:
        return "".join("\n" if char == "\n" else " " for char in match.group(0))

    return pattern.sub(blank, text)


def header_guard(text: str, path: Path) -> str | None:
    if path.suffix.lower() not in HEADER_SUFFIXES:
        return None
    directives = list(DIRECTIVE_RE.finditer(strip_comments_and_literals(text)))
    if (
        len(directives) >= 2
        and directives[0].group(1) == "ifndef"
        and directives[1].group(1) == "define"
        and directives[0].group(2) == directives[1].group(2)
    ):
        return directives[0].group(2)
    return None


def macro_definitions(text: str, path: Path) -> list[tuple[str, int]]:
    guard = header_guard(text, path)
    return [
        (match.group(1), match.start())
        for match in MACRO_DEFINE_RE.finditer(text)
        if match.group(1) != guard
    ]


def is_macro_header(path: Path, root: Path, layer: str) -> bool:
    """允许层级 layer_def.h，或任意模块目录内与目录同名的 *_def.h。"""

    if path == root / LOGGER_MACRO_HEADER:
        return True
    relative = path.relative_to(root / "src" / layer)
    if len(relative.parts) == 1:
        return relative.name == f"{layer}_def.h"
    return relative.name == f"{relative.parent.name}_def.h"


def module_macro_root(path: Path, root: Path, layer: str) -> Path | None:
    """模块级 *_def.h 返回模块根目录；层级 *_def.h 返回 None。"""

    relative = path.relative_to(root / "src" / layer)
    if len(relative.parts) > 1 and relative.name == f"{relative.parent.name}_def.h":
        return path.parent
    return None


def resolve_local_include(include: str, source: Path, root: Path) -> Path | None:
    candidates = [source.parent / include, root / include, root / "src" / include]
    candidates.extend(root / "src" / layer / include for layer in sorted(LAYERS))
    candidates.append(root / "src" / "include" / include)
    for candidate in candidates:
        candidate = candidate.resolve()
        try:
            candidate.relative_to(root)
        except ValueError:
            continue
        if candidate.is_file():
            return candidate
    return None


def targets_src_include(include: str, source: Path, root: Path) -> bool:
    include_root = (root / "src" / "include").resolve()
    candidates = (source.parent / include, root / include, include_root / include)
    return SRC_INCLUDE_RE.search(include) is not None or any(
        candidate.is_file() and candidate.resolve().is_relative_to(include_root)
        for candidate in candidates
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, add_help=False)
    parser._positionals.title = "位置参数"
    parser._optionals.title = "选项"
    parser.add_argument("-h", "--help", action="help", help="显示帮助信息并退出")
    parser.add_argument(
        "paths", nargs="*", help="要检查的文件或目录；默认检查全部受控分层"
    )
    parser.add_argument("--root", default=".", help="仓库根目录（默认：当前目录）")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    violations: list[tuple[Path, int, str]] = []
    checked = 0

    for path in iter_files(root, args.paths):
        source_layer = layer_of(path, root)
        if source_layer is None:
            continue
        checked += 1
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            violations.append((path, 1, f"无法读取文件：{exc}"))
            continue

        for match in INCLUDE_RE.finditer(text):
            include = match.group(1)
            target_match = LAYER_INCLUDE_RE.search(include)
            if target_match:
                target_layer = target_match.group(1)
                if target_layer not in ALLOWED[source_layer]:
                    violations.append(
                        (
                            path,
                            line_number(text, match.start()),
                            f"禁止的依赖 {source_layer} -> {target_layer}：{include}",
                        )
                    )
                elif source_layer == target_layer == "drivers":
                    source_parts = path.relative_to(root / "src" / "drivers").parts
                    target_parts = Path(include[target_match.end() :].lstrip("/")).parts
                    if (
                        source_parts
                        and target_parts
                        and source_parts[0] != target_parts[0]
                    ):
                        violations.append(
                            (
                                path,
                                line_number(text, match.start()),
                                f"驱动单元 {source_parts[0]} 依赖了同级驱动单元 {target_parts[0]}：{include}",
                            )
                        )
                if target_layer == source_layer and Path(include).name.endswith(
                    "_def.h"
                ):
                    macro_header = resolve_local_include(include, path, root)
                    if macro_header is not None:
                        module_root = module_macro_root(
                            macro_header, root, source_layer
                        )
                        if module_root is not None and not path.is_relative_to(
                            module_root
                        ):
                            violations.append(
                                (
                                    path,
                                    line_number(text, match.start()),
                                    f"模块宏头只能在 {module_root.relative_to(root)} 内使用：{include}",
                                )
                            )
            if source_layer == "app" and targets_src_include(include, path, root):
                violations.append(
                    (
                        path,
                        line_number(text, match.start()),
                        f"app 禁止包含 src/include：{include}",
                    )
                )
            if source_layer != "app" and LVGL_INCLUDE_RE.search(include):
                violations.append(
                    (
                        path,
                        line_number(text, match.start()),
                        f"{source_layer} 禁止包含 LVGL：{include}",
                    )
                )

        if source_layer != "app":
            for match in LVGL_TYPE_RE.finditer(text):
                violations.append(
                    (
                        path,
                        line_number(text, match.start()),
                        f"{source_layer} 禁止出现 LVGL 类型：{match.group(0)}",
                    )
                )

        definitions = macro_definitions(text, path)
        if path.suffix.lower() in HEADER_SUFFIXES and not is_macro_header(
            path, root, source_layer
        ):
            for name, offset in definitions:
                violations.append(
                    (
                        path,
                        line_number(text, offset),
                        f"宏 {name} 应放入层级或模块级 *_def.h；普通头文件只声明类型和 API",
                    )
                )

    for path, line, message in sorted(
        violations, key=lambda item: (str(item[0]), item[1], item[2])
    ):
        try:
            display = path.relative_to(root)
        except ValueError:
            display = path
        print(f"{display}:{line}: {message}")

    if violations:
        print(
            f"架构检查失败：在 {checked} 个文件中发现 {len(violations)} 个违规",
            file=sys.stderr,
        )
        return 1
    print(f"架构检查通过：已检查 {checked} 个文件")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
