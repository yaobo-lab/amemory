# Books

每本电子书使用一个独立目录，目录名为小写字母、数字和单连字符组成的稳定名称：

```text
resource/books/
└── rust-book/
    ├── BOOK.md
    └── chapters/
        ├── 01-introduction.md
        └── 02-ownership.md
```

`BOOK.md` 必须包含以下 frontmatter：

```markdown
---
name: rust-book
title: Rust 程序设计语言
description: Rust 学习资料
---
```

`name` 必须与目录名一致。`description` 和 `author` 可省略；目录中的 UTF-8 Markdown 文件会同步到 Memvid，其他文件只保留在磁盘中。
