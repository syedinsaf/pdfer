<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->

<div align="center">

# pdfer

Merge and split PDF files with predictable behavior, strong safety guarantees, and a clean command-line interface.

[![Crates.io](https://img.shields.io/crates/v/pdfer?style=for-the-badge\&logo=rust\&logoColor=white\&label=crates.io\&color=rust)](https://crates.io/crates/pdfer)
[![GitHub release](https://img.shields.io/github/v/release/syedinsaf/pdfer?style=for-the-badge\&logo=github\&logoColor=white\&color=rust)](https://github.com/syedinsaf/pdfer/releases)
[![Downloads](https://img.shields.io/github/downloads/syedinsaf/pdfer/total?style=for-the-badge&logo=github&logoColor=white&color=rust)](https://github.com/syedinsaf/pdfer/releases)
[![License](https://img.shields.io/github/license/syedinsaf/pdfer?style=for-the-badge\&logo=github\&logoColor=white\&color=rust)](LICENSE)

[Download](https://github.com/syedinsaf/pdfer/releases) •
[Quick Start](#quick-start) •
[Usage](#basic-usage)

</div>

---

## Table of Contents

* [Overview](#overview)
* [Design Goals](#design-goals)
* [Quick Start](#quick-start)
* [Basic Usage](#basic-usage)
* [Page Range Syntax](#page-range-syntax)
* [Safety Features](#safety-features)
* [Building from Source](#building-from-source)
* [Contributing](#contributing)
* [License](#license)
* [Disclaimer](#disclaimer)

---

## Overview

**pdfer** is a small, focused, and reliable PDF utility written in Rust.

It provides two core capabilities:

* **Merging PDFs** into a single document
* **Splitting PDFs** into per-page files or selected ranges

The tool prioritizes correctness, safety, and predictable behavior over
feature bloat.

Unlike many GUI-based utilities, pdfer is designed for:

* developers
* scripting workflows
* automation pipelines
* low-overhead environments

---

## Design Goals

pdfer intentionally keeps its scope narrow and dependable:

* Deterministic behavior
* No silent data corruption
* Strict input validation
* Safe output handling
* Cross-platform portability
* Minimal dependencies

The tool refuses to proceed when encountering ambiguous or unsafe states
(e.g., invalid page ranges, non-PDF files, overwrite conflicts).

---

## Quick Start

### Prebuilt Binaries

Download platform-specific binaries from:

[https://github.com/syedinsaf/pdfer/releases](https://github.com/syedinsaf/pdfer/releases)

Example:

```bash
pdfer merge a.pdf b.pdf -o merged.pdf
pdfer split document.pdf 1,3,5-10
```

---

## Basic Usage

### Show PDF Info

```bash
pdfer file.pdf
pdfer *.pdf
pdfer -r ./documents
```

Displays:

* page count
* PDF version
* metadata (title / author / subject if present)

---

### Merge PDFs

```bash
pdfer merge a.pdf b.pdf -o merged.pdf
pdfer m *.pdf -o combined.pdf
```

Behavior:

* preserves page order
* validates inputs
* refuses empty PDFs

---

### Split PDFs

Split all pages:

```bash
pdfer split document.pdf
```

Split selected pages:

```bash
pdfer split document.pdf 1,3,5-10
```

Custom output folder:

```bash
pdfer split document.pdf 1-5 -o output_pages
```

---

## Page Range Syntax

pdfer supports flexible page selection:

| Syntax  | Meaning             |
| ------- | ------------------- |
| `5`     | Single page         |
| `1,3,7` | Specific pages      |
| `2-6`   | Inclusive range     |
| `10-`   | From page 10 to end |

Rules:

* Page numbers start at **1**
* Ranges are validated strictly
* Invalid specifications fail early

Examples:

```bash
pdfer split file.pdf 1-3
pdfer split file.pdf 4,7,9-12
pdfer split file.pdf 5-
```

---

## Safety Features

pdfer is designed to avoid destructive mistakes:

✔ Overwrite protection
✔ Interactive conflict resolution
✔ Strict file type validation
✔ Refusal on invalid ranges
✔ No partial writes on failure

When an output file or directory already exists, pdfer will prompt for:

* overwrite
* rename
* abort

---

## Building from Source

### Requirements

* Rust (stable toolchain)
* Git

### Build

```bash
git clone https://github.com/syedinsaf/pdfer.git
cd pdfer
cargo build --release
```

Binary output:

* Linux / macOS → `target/release/pdfer`
* Windows → `target/release/pdfer.exe`

---

## Contributing

Bug reports and improvements are welcome.

Useful information for issues:

* OS and version
* Rust version
* Example PDFs (if possible)
* Exact command used
* Full error output

Pull requests should preserve:

* safety guarantees
* deterministic behavior
* portability

---

## License

pdfer is licensed under the **Apache License 2.0**.

See `LICENSE` for details.

---

## Disclaimer

Use at your own risk.

Always verify important documents after processing.

The author is not responsible for:

* data loss
* corrupted documents
* workflow disruptions

---
