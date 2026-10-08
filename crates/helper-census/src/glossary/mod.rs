// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One native glossary gate. The selected glossary supplies every rule/test;
//! the root's default catalogue supplies the required real standardized-spelling
//! specimens, even when `--po` selects an external catalogue for scanning.

mod pattern;
mod po;
mod surface;

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use pattern::{Pattern, literal};

const GLOSSARY: &str = "docs/book/po/glossary-zh-Hans.md";
const CATALOGUE: &str = "docs/book/po/zh-Hans.po";
const HEADER: [&str; 7] = [
    "#",
    "Term",
    "Anchor",
    "Rendering",
    "Basis",
    "Rejected",
    "Note",
];
const UNRELATED: &str = "The weather was fine today.";

#[derive(Debug)]
struct Rejection {
    spelling: String,
    pattern: Pattern,
}

#[derive(Debug)]
struct Row {
    source: PathBuf,
    term: String,
    anchors: Vec<String>,
    anchor_patterns: Vec<Pattern>,
    token_patterns: Vec<Pattern>,
    rendering: String,
    keep: bool,
    rejected: Vec<Rejection>,
    note: String,
    line: usize,
}

impl Row {
    fn global(&self) -> bool {
        self.anchors.is_empty()
    }
    fn anchored(&self, visible: &str) -> Result<bool, String> {
        for pattern in &self.anchor_patterns {
            if pattern.matches(visible)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn bare(cell: &str) -> &str {
    let cell = cell.trim();
    cell.strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .unwrap_or(cell)
}

fn list(cell: &str) -> Vec<String> {
    cell.split('、')
        .map(bare)
        .filter(|s| !matches!(*s, "" | "—" | "-" | "–"))
        .map(str::to_owned)
        .collect()
}

fn split_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut buf = String::new();
    let mut code = false;
    let mut chars = line.trim().chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            buf.push('|');
            chars.next();
        } else if ch == '`' {
            code = !code;
            buf.push(ch);
        } else if ch == '|' && !code {
            cells.push(buf.trim().to_owned());
            buf.clear();
        } else {
            buf.push(ch);
        }
    }
    cells.push(buf.trim().to_owned());
    if cells.first().is_some_and(String::is_empty) {
        cells.remove(0);
    }
    if cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

fn regex_body(text: &str) -> Option<&str> {
    text.strip_prefix('/').and_then(|s| s.strip_suffix('/'))
}

fn parse_glossary(text: &str, path: &Path) -> Result<Vec<Row>, String> {
    let lines: Vec<_> = text.lines().collect();
    let header = lines
        .iter()
        .position(|line| line.trim_start().starts_with('|') && split_cells(line) == HEADER)
        .ok_or("glossary has no seven-column header")?;
    let separator = lines
        .get(header + 1)
        .ok_or("glossary has no separator row")?;
    let separators = split_cells(separator);
    if separators.len() != 7
        || separators.iter().any(|cell| {
            !cell.contains('-') || !cell.bytes().all(|byte| matches!(byte, b'-' | b':' | b' '))
        })
    {
        return Err("malformed glossary separator".to_owned());
    }
    let mut rows = Vec::new();
    for (index, line) in lines.iter().enumerate().skip(header + 2) {
        if !line.trim_start().starts_with('|') {
            break;
        }
        let cells = split_cells(line);
        if cells.len() != 7 {
            return Err(format!(
                "glossary:{}: expected seven cells, got {}",
                index + 1,
                cells.len()
            ));
        }
        if cells[1].is_empty() || cells[3].is_empty() {
            return Err(format!("glossary:{}: empty term/rendering", index + 1));
        }
        let anchors = list(&cells[2]);
        let keep = cells[4].starts_with('K');
        let mut anchor_patterns = Vec::new();
        let mut token_patterns = Vec::new();
        for anchor in &anchors {
            if keep && regex_body(anchor).is_some() {
                return Err(format!(
                    "glossary:{} row {:?}: K rows require literal anchors, not {anchor:?}",
                    index + 1,
                    cells[1]
                ));
            }
            let source = regex_body(anchor).map_or_else(|| literal(anchor), str::to_owned);
            anchor_patterns.push(
                Pattern::anchor(&source).map_err(|error| {
                    format!("glossary:{} row {:?}: {error}", index + 1, cells[1])
                })?,
            );
            if keep {
                token_patterns.push(
                    Pattern::new(
                        &format!("(?<![A-Za-z0-9]){}(?![A-Za-z0-9])", literal(anchor)),
                        false,
                    )
                    .map_err(|error| {
                        format!("glossary:{} row {:?}: {error}", index + 1, cells[1])
                    })?,
                );
            }
        }
        let rejected = list(&cells[5])
            .into_iter()
            .map(|spelling| {
                let source =
                    regex_body(&spelling).map_or_else(|| literal(&spelling), str::to_owned);
                Ok(Rejection {
                    pattern: Pattern::new(&source, false).map_err(|error| {
                        format!("glossary:{} row {:?}: {error}", index + 1, cells[1])
                    })?,
                    spelling,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        rows.push(Row {
            source: path.to_path_buf(),
            term: cells[1].clone(),
            anchors,
            anchor_patterns,
            token_patterns,
            rendering: cells[3].clone(),
            keep,
            rejected,
            note: cells[6].clone(),
            line: index + 1,
        });
    }
    if rows.is_empty() {
        return Err("glossary has no rows".to_owned());
    }
    Ok(rows)
}

fn specimen(rule: &Rejection) -> Result<String, String> {
    let Some(body) = regex_body(&rule.spelling) else {
        return Ok(rule.spelling.clone());
    };
    let bytes = body.as_bytes();
    let mut out = String::new();
    let mut pos = 0;
    while pos < bytes.len() {
        if body[pos..].starts_with("(?") {
            let mut depth = 1;
            let mut class = false;
            pos += 2;
            while pos < bytes.len() && depth > 0 {
                if bytes[pos] == b'\\' {
                    pos += 2;
                    continue;
                }
                if bytes[pos] == b'[' {
                    class = true;
                }
                if bytes[pos] == b']' {
                    class = false;
                }
                if !class && bytes[pos] == b'(' {
                    depth += 1;
                }
                if !class && bytes[pos] == b')' {
                    depth -= 1;
                }
                pos += 1;
            }
            if depth != 0 {
                return Err(format!("cannot derive specimen for {}", rule.spelling));
            }
        } else {
            let ch = body[pos..].chars().next().expect("in range");
            out.push(ch);
            pos += ch.len_utf8();
        }
    }
    Ok(out
        .replace(r"\d+", "0")
        .replace(r"\d", "0")
        .replace(r"\s*", " ")
        .replace(r"\s+", " "))
}

fn consistency(rows: &[Row]) -> Result<(), String> {
    for row in rows {
        if row.global() && !row.rejected.is_empty() && !row.note.to_lowercase().contains("global") {
            return Err(format!(
                "{}:{}: global rejection needs a GLOBAL reason",
                row.term, row.line
            ));
        }
        if row.global() && row.keep {
            return Err(format!("{}:{}: K row needs anchors", row.term, row.line));
        }
        for rule in &row.rejected {
            if !rule.pattern.matches(&specimen(rule)?)? {
                return Err(format!(
                    "{}: derived specimen does not match {}",
                    row.term, rule.spelling
                ));
            }
            for other in rows {
                if rule.pattern.matches(bare(&other.rendering))? {
                    return Err(format!(
                        "{} rejects its own table: {} in {}",
                        row.term, rule.spelling, other.term
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Offence {
    row: usize,
    token: Option<usize>,
    message: String,
}

fn offences(
    label: &str,
    id: Option<&str>,
    translated: &str,
    rows: &[Row],
) -> Result<Vec<Offence>, String> {
    let source = id.map(surface::project);
    let target = surface::project(translated);
    let mut out = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row.global()
            || source
                .as_ref()
                .map(|s| row.anchored(&s.visible))
                .transpose()
                .map_err(|error| format!("{label}: anchor row {:?}: {error}", row.term))?
                .unwrap_or(false)
        {
            for rule in &row.rejected {
                if rule
                    .pattern
                    .matches(&target.prose)
                    .map_err(|error| format!("{label}: rejection row {:?}: {error}", row.term))?
                {
                    // File rules are GLOBAL and their current patterns are line
                    // local. Identify the actual containing line, not a guessed
                    // match span (the shared matcher deliberately returns bool).
                    let location = if id.is_none() {
                        let mut line = None;
                        for (index, text) in target.prose.lines().enumerate() {
                            if rule.pattern.matches(text).map_err(|error| {
                                format!(
                                    "{label}:{}: rejection row {:?}: {error}",
                                    index + 1,
                                    row.term
                                )
                            })? {
                                line = Some(index + 1);
                                break;
                            }
                        }
                        line.map_or_else(|| label.to_owned(), |line| format!("{label}:{line}"))
                    } else {
                        label.to_owned()
                    };
                    out.push(Offence {
                        row: row_index,
                        token: None,
                        message: format!(
                            "{location}: rejected rule {:?} for {:?}; rendering {:?} ({}:{})",
                            rule.spelling,
                            row.term,
                            row.rendering,
                            row.source.display(),
                            row.line
                        ),
                    });
                }
            }
        }
        if row.keep
            && let Some(source) = &source
        {
            for (token_index, pattern) in row.token_patterns.iter().enumerate() {
                let source_has = pattern
                    .matches(&source.visible)
                    .map_err(|error| format!("{label}: source K row {:?}: {error}", row.term))?;
                let target_has = pattern.matches(&target.visible).map_err(|error| {
                    format!("{label}: translated K row {:?}: {error}", row.term)
                })?;
                if source_has && !target_has {
                    out.push(Offence { row: row_index, token: Some(token_index), message: format!("{label}: keep-English token {:?} does not survive visibly; row {:?} ({}:{})", row.anchors[token_index], row.term, row.source.display(), row.line) });
                }
            }
        }
    }
    Ok(out)
}

fn is_translation(text: &str) -> bool {
    let nonspace = text.chars().filter(|ch| !ch.is_whitespace()).count();
    let cjk = text.chars().filter(|ch| matches!(u32::from(*ch), 0x3000..=0x303f | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0xff00..=0xffef)).count();
    cjk * 100 >= nonspace * 15
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn tracked_files(root: &Path, glossary: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|error| format!("git ls-files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut paths = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    let mut selected = Vec::new();
    for bytes in paths {
        if !bytes.ends_with(b".md") {
            continue;
        }
        #[cfg(unix)]
        let rel = {
            use std::os::unix::ffi::OsStrExt;
            std::ffi::OsStr::from_bytes(bytes)
        };
        #[cfg(not(unix))]
        let rel = std::ffi::OsStr::new(
            std::str::from_utf8(bytes)
                .map_err(|error| format!("tracked Markdown path is not UTF-8: {error}"))?,
        );
        let path = root.join(rel);
        if path == glossary || path == root.join(GLOSSARY) || !path.is_file() {
            continue;
        }
        if is_translation(&read(&path)?) {
            selected.push(path);
        }
    }
    Ok(selected)
}

#[derive(Debug)]
struct Options {
    root: PathBuf,
    po: PathBuf,
    glossary: PathBuf,
    self_test: bool,
    inventory: Option<PathBuf>,
}

fn options(args: Vec<String>) -> Result<Options, String> {
    let mut root = purrdf_testkit::paths::workspace_root();
    let mut po = None;
    let mut glossary = None;
    let mut self_test = false;
    let mut inventory = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--glossary-gate" => {}
            "--root" => root = PathBuf::from(args.next().ok_or("--root needs a path")?),
            "--po" => po = Some(PathBuf::from(args.next().ok_or("--po needs a path")?)),
            "--glossary" => {
                glossary = Some(PathBuf::from(args.next().ok_or("--glossary needs a path")?));
            }
            "--inventory" => {
                inventory = Some(PathBuf::from(
                    args.next().ok_or("--inventory needs a path")?,
                ));
            }
            "--self-test" => self_test = true,
            _ => return Err(format!("unknown glossary option {arg:?}")),
        }
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("root: {error}"))?;
    Ok(Options {
        po: po.unwrap_or_else(|| root.join(CATALOGUE)),
        glossary: glossary.unwrap_or_else(|| root.join(GLOSSARY)),
        root,
        self_test,
        inventory,
    })
}

pub(super) fn run(args: Vec<String>) -> Result<(), String> {
    let options = options(args)?;
    let rows = parse_glossary(&read(&options.glossary)?, &options.glossary)
        .map_err(|error| format!("{}: {error}", options.glossary.display()))?;
    consistency(&rows)?;
    let authority = po::parse(&read(&options.root.join(CATALOGUE))?)
        .map_err(|error| format!("{}: {error}", options.root.join(CATALOGUE).display()))?;
    let tests = self_test(&rows, &authority)?;
    let rejections: usize = rows.iter().map(|row| row.rejected.len()).sum();
    let tokens: usize = rows.iter().map(|row| row.token_patterns.len()).sum();
    if options.self_test {
        println!(
            "OK: {} glossary rows, {rejections} rejections, {tokens} K tokens, {tests} production-path self-tests",
            rows.len()
        );
        return Ok(());
    }
    let entries = po::parse(&read(&options.po)?)
        .map_err(|error| format!("{}: {error}", options.po.display()))?;
    let files = tracked_files(&options.root, &options.glossary)?;
    let mut found = Vec::new();
    let mut units = 0;
    for entry in entries.iter().filter(|entry| entry.active) {
        units += 1;
        found.extend(offences(
            &format!("{}:{}", options.po.display(), entry.line),
            Some(&entry.id),
            &entry.translated,
            &rows,
        )?);
    }
    for path in &files {
        let text = read(path)?;
        units += text.lines().count();
        found.extend(offences(&path.display().to_string(), None, &text, &rows)?);
    }
    let inventory = sweep(&entries, &rows)?;
    if let Some(path) = options.inventory {
        std::fs::write(&path, inventory).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    if !found.is_empty() {
        return Err(found
            .into_iter()
            .map(|offence| offence.message)
            .collect::<Vec<_>>()
            .join("\n"));
    }
    println!(
        "OK: {} rows, {rejections} rejections, {tokens} K tokens respected by {units} translated units; {} tracked translated Markdown documents; {tests} self-tests",
        rows.len(),
        files.len()
    );
    Ok(())
}

fn sweep(entries: &[po::Entry], rows: &[Row]) -> Result<String, String> {
    let mut inventory = String::from(
        "row\tterm\tanchored_units\traw_specimen_occurrences\trefused_units\tpoison_controls\n",
    );
    let mut contexts = String::from(
        "\n# Raw substrings in anchored visible Chinese prose (including accepted lookaround neighbours)\n",
    );
    for (index, row) in rows
        .iter()
        .enumerate()
        .filter(|(_, row)| !row.rejected.is_empty())
    {
        let mut anchored = 0;
        let mut raw = 0;
        let mut refused = 0;
        let mut poison = 0;
        let mut specimen_entry = None;
        for entry in entries.iter().filter(|entry| entry.active) {
            if row.global() || row.anchored(&surface::project(&entry.id).visible)? {
                anchored += 1;
                specimen_entry.get_or_insert(entry);
                let prose = surface::project(&entry.translated).prose;
                for rule in &row.rejected {
                    let sample = specimen(rule)?;
                    for (offset, _) in prose.match_indices(&sample) {
                        raw += 1;
                        let mut left: Vec<_> = prose[..offset].chars().rev().take(32).collect();
                        left.reverse();
                        let context: String = left
                            .into_iter()
                            .chain(prose[offset..].chars().take(sample.chars().count() + 32))
                            .collect();
                        writeln!(
                            contexts,
                            "row {}\tPO line {}\t{:?}\tcontext {:?}",
                            index + 1,
                            entry.line,
                            rule.spelling,
                            context
                        )
                        .expect("String write");
                    }
                    if rule.pattern.matches(&prose)? {
                        refused += 1;
                    }
                }
            }
        }
        if let Some(entry) = specimen_entry {
            for rule in &row.rejected {
                let text = format!("{}\n{}", entry.translated, specimen(rule)?);
                if !offences("real paragraph poison", Some(&entry.id), &text, rows)?
                    .iter()
                    .any(|offence| offence.row == index && offence.token.is_none())
                {
                    return Err(format!(
                        "real paragraph poison not refused for {} / {} at line {}",
                        row.term, rule.spelling, entry.line
                    ));
                }
                poison += 1;
            }
        }
        writeln!(
            inventory,
            "{}\t{}\t{anchored}\t{raw}\t{refused}\t{poison}",
            index + 1,
            row.term
        )
        .expect("String write");
    }
    inventory.push_str(&contexts);
    Ok(inventory)
}

const NEIGHBOURS: &[(&str, &str)] = &[
    ("具名图", "作者具名发表了这篇文章。"),
    (r"/(?<!参考)资料集/", "参考资料见附录。"),
    ("资料类型", "参考资料见附录。"),
    ("空白节点", "网页模板中的空白节点会被忽略。"),
    ("字面值", "该常量的字面值为 42。"),
    ("语言标记", "编辑器根据语言标记进行语法高亮。"),
    ("本体论", "本体论是哲学的一个分支。"),
    (r"/知识图(?!谱)/", "这张知识图示意了课程结构。"),
    ("蕴含", "这一设计蕴含着一个假设。"),
    ("实体化", "该抽象概念被实体化为一个类。"),
    ("标准化", "RDF 1.2 已由 W3C 标准化。"),
    (r"/决定性(?!能)/", "这是决定性因素。"),
    (r"/(?<!输)出处/", "引文出处见脚注。"),
    ("三元组术语", "本节解释三元组术语的由来。"),
    ("三元组词项", "三元组词项在逻辑学教材中另有含义。"),
    ("基本方向", "设计的基本方向是确定性。"),
    ("组合数据类型", "C 语言中的结构体是一种组合数据类型。"),
    (r"/(?<!台)账本/", "区块链是一种分布式账本。"),
    ("核外", "核外电子决定了元素的化学性质。"),
    (
        r"/研究对象(?!\s*[（(]\s*Research Object)/",
        "本研究的研究对象为大学生。",
    ),
    (r"/表面(?!上|看来|来看)/", "水的表面张力很大。"),
    ("铸造", "青铜器由铸造而成。"),
    ("抵达", "列车准时抵达车站。"),
    ("大声", "请勿大声喧哗。"),
    (r"/显式报错(?!失败)/", "表单填写有误时，页面会显式报错。"),
    (r"/(?<!丢弃并)显式告知/", "合同条款的变更须向客户显式告知。"),
    ("全文搜索", "本站提供全文搜索功能。"),
    ("校验报告", "文件校验报告显示哈希一致。"),
    (r"/基础\s*IRI/", "入门课程先讲基础 IRI 语法，再讲进阶内容。"),
    ("以失败关闭", "这个项目最终以失败关闭，团队随后解散。"),
    ("封闭失败", "阀门封闭失败，导致管道泄漏。"),
    ("预言机", "区块链预言机把链下数据写入智能合约。"),
    ("一致性", "SHACL 1.2 一致性测试全部通过。"),
    ("制品", "这件青铜制品出土于商代遗址。"),
    (r"/内置(?!函数)/", "Python 内置了许多常用模块。"),
    ("准备阶段", "考试准备阶段要注意休息。"),
    ("准备时", "出门准备时别忘了带伞。"),
    ("格式良好", "这份简历格式良好，便于阅读。"),
    ("偏移", "该字段记录文件内的字节偏移。"),
    ("写入器", "这台光盘写入器已经停产。"),
    ("凭证", "登录时请出示有效的身份凭证。"),
    ("上下文视角", "从用户的上下文视角来看，这个设计很直观。"),
];

fn po_quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn probe(id: &str, translated: &str, rows: &[Row]) -> Result<Vec<Offence>, String> {
    let text = format!("msgid {}\nmsgstr {}\n", po_quote(id), po_quote(translated));
    let mut found = Vec::new();
    for entry in po::parse(&text)?.into_iter().filter(|entry| entry.active) {
        found.extend(offences(
            "self-test",
            Some(&entry.id),
            &entry.translated,
            rows,
        )?);
    }
    Ok(found)
}

fn verdict(
    id: &str,
    text: &str,
    must_refuse: bool,
    what: &str,
    rows: &[Row],
    count: &mut usize,
) -> Result<(), String> {
    let found = probe(id, text, rows)?;
    *count += 1;
    if found.is_empty() == must_refuse {
        return Err(format!(
            "self-test {what}: {} ({id:?} / {text:?}): {found:?}",
            if must_refuse {
                "NOT REFUSED"
            } else {
                "FALSELY REFUSED"
            }
        ));
    }
    Ok(())
}

fn surviving_other_tokens(id: &str, rows: &[Row], omit: Option<&str>) -> Result<String, String> {
    let visible = surface::project(id).visible;
    let mut out = String::new();
    for row in rows.iter().filter(|row| row.keep) {
        for (token, pattern) in row.anchors.iter().zip(&row.token_patterns) {
            if Some(token.as_str()) != omit && pattern.matches(&visible)? {
                out.push_str(" （");
                out.push_str(token);
                out.push_str("） ");
            }
        }
    }
    Ok(out)
}

fn token_verdict(
    row: usize,
    token: usize,
    id: &str,
    text: &str,
    must_refuse: bool,
    rows: &[Row],
    count: &mut usize,
) -> Result<(), String> {
    let other = surviving_other_tokens(id, rows, Some(&rows[row].anchors[token]))?;
    let found = probe(id, &format!("{text} {other}"), rows)?;
    let target_found = found
        .iter()
        .any(|offence| offence.row == row && offence.token == Some(token));
    *count += 1;
    if target_found != must_refuse
        || found
            .iter()
            .any(|offence| offence.row != row || offence.token != Some(token))
    {
        return Err(format!(
            "self-test isolated K row {:?}, token {:?}: expected refusal={must_refuse}, got {found:?}; source={id:?}, target={text:?}",
            rows[row].term, rows[row].anchors[token]
        ));
    }
    Ok(())
}

fn self_test(rows: &[Row], authority: &[po::Entry]) -> Result<usize, String> {
    let mut count = 0;
    if !rows.iter().any(|row| !row.rejected.is_empty()) {
        return Err("glossary has no rejection to exercise".to_owned());
    }
    for row in rows {
        if row.anchored(UNRELATED)? {
            return Err(format!("unrelated self-test is anchored by {}", row.term));
        }
    }
    anchor_controls(rows, &mut count)?;
    for row in rows {
        let anchored = if row.global() {
            UNRELATED.to_owned()
        } else {
            format!("This paragraph is about {}.", row.anchors[0])
        };
        for rule in &row.rejected {
            let specimen = specimen(rule)?;
            let neighbour = NEIGHBOURS
                .iter()
                .find(|(spelling, _)| *spelling == rule.spelling)
                .map(|(_, neighbour)| *neighbour)
                .ok_or_else(|| format!("{}: no neighbour for {}", row.term, rule.spelling))?;
            if rule.pattern.matches(neighbour)? == row.global() {
                return Err(format!(
                    "{}: neighbour does not prove other sense / global near miss",
                    row.term
                ));
            }
            let keep = surviving_other_tokens(&anchored, rows, None)?;
            verdict(
                &anchored,
                &format!("本页使用{specimen}一词。{keep}"),
                true,
                &format!("{} refuses {}", row.term, rule.spelling),
                rows,
                &mut count,
            )?;
            verdict(
                UNRELATED,
                neighbour,
                false,
                "ordinary neighbour",
                rows,
                &mut count,
            )?;
            verdict(
                &anchored,
                &format!("不要写 `{specimen}`。{keep}"),
                false,
                "code exemption",
                rows,
                &mut count,
            )?;
            // Hidden destinations/titles cannot activate a paragraph anchor,
            // and cannot introduce rejected prose into a translation.
            if !row.global() {
                let hidden_id = format!(
                    "[ordinary label](https://example.invalid \"{}\")",
                    row.anchors[0]
                );
                let other = surviving_other_tokens(&hidden_id, rows, None)?;
                verdict(
                    &hidden_id,
                    &format!("{specimen}{other}"),
                    false,
                    "hidden URL/title anchor",
                    rows,
                    &mut count,
                )?;
                let shown_id = format!("[{}](https://example.invalid)", row.anchors[0]);
                let keep = surviving_other_tokens(&shown_id, rows, None)?;
                verdict(
                    &shown_id,
                    &format!("本页使用{specimen}一词。{keep}"),
                    true,
                    "visible label anchor",
                    rows,
                    &mut count,
                )?;
                for marker in ["", "!"] {
                    let hidden_id = format!(
                        "{marker}[ordinary\nlabel](\nhttps://example.invalid\n\"{}\nordinary title\"\n)",
                        row.anchors[0]
                    );
                    verdict(
                        &hidden_id,
                        &format!("本页使用{specimen}一词。"),
                        false,
                        "multiline hidden anchor/title",
                        rows,
                        &mut count,
                    )?;
                    let visible_id = format!(
                        "{marker}[{}\nvisible label](\nhttps://example.invalid\n)",
                        row.anchors[0]
                    );
                    let keep = surviving_other_tokens(&visible_id, rows, None)?;
                    verdict(
                        &visible_id,
                        &format!("本页使用{specimen}一词。{keep}"),
                        true,
                        "multiline visible anchor label",
                        rows,
                        &mut count,
                    )?;
                }
            }
            verdict(
                &anchored,
                &format!("[普通标签](https://example.invalid \"{specimen}\"){keep}"),
                false,
                "hidden translated rejection",
                rows,
                &mut count,
            )?;
            for marker in ["", "!"] {
                verdict(
                    &anchored,
                    &format!(
                        "{marker}[普通\n标签](\nhttps://example.invalid\n\"{specimen}\n普通标题\"\n){keep}"
                    ),
                    false,
                    "multiline hidden translated rejection",
                    rows,
                    &mut count,
                )?;
            }
        }
        if !row.rejected.is_empty() && bare(&row.rendering) != "as written" {
            let keep = surviving_other_tokens(&anchored, rows, None)?;
            verdict(
                &anchored,
                &format!("本页使用 {} 一词。{keep}", bare(&row.rendering)),
                false,
                "own rendering",
                rows,
                &mut count,
            )?;
        }
    }
    for (row_index, row) in rows.iter().enumerate().filter(|(_, row)| row.keep) {
        for (token_index, token) in row.anchors.iter().enumerate() {
            let id = format!("The {token} toolkit is here.");
            token_verdict(
                row_index,
                token_index,
                &id,
                "该工具包在此。",
                true,
                rows,
                &mut count,
            )?;
            token_verdict(
                row_index,
                token_index,
                &id,
                &format!("该 {token} 工具包在此。"),
                false,
                rows,
                &mut count,
            )?;
            for embedded in [format!("X{token}"), format!("{token}X")] {
                token_verdict(
                    row_index,
                    token_index,
                    &format!("The {embedded} toolkit."),
                    "该工具包。",
                    false,
                    rows,
                    &mut count,
                )?;
                token_verdict(
                    row_index,
                    token_index,
                    &id,
                    &embedded,
                    true,
                    rows,
                    &mut count,
                )?;
            }
            let near_case = if token.to_lowercase() == *token {
                token.to_uppercase()
            } else {
                token.to_lowercase()
            };
            token_verdict(
                row_index,
                token_index,
                &near_case,
                "该工具包。",
                false,
                rows,
                &mut count,
            )?;
            token_verdict(
                row_index,
                token_index,
                &id,
                &near_case,
                true,
                rows,
                &mut count,
            )?;
            for wrapped in [
                format!("({token})"),
                format!("「{token}」"),
                format!("中{token}文"),
                format!("`{token}`"),
                format!("[{token}](https://example.invalid)"),
            ] {
                token_verdict(
                    row_index,
                    token_index,
                    &wrapped,
                    &wrapped,
                    false,
                    rows,
                    &mut count,
                )?;
                token_verdict(
                    row_index,
                    token_index,
                    &wrapped,
                    "该工具包。",
                    true,
                    rows,
                    &mut count,
                )?;
            }
            let hidden = format!("[ordinary](https://example.invalid \"{token}\")");
            token_verdict(
                row_index,
                token_index,
                &hidden,
                "该工具包。",
                false,
                rows,
                &mut count,
            )?;
            token_verdict(row_index, token_index, &id, &hidden, true, rows, &mut count)?;
            let definition = format!("[ordinary][ref]\n[ref]: https://example.invalid \"{token}\"");
            token_verdict(
                row_index,
                token_index,
                &definition,
                "该工具包。",
                false,
                rows,
                &mut count,
            )?;
            token_verdict(
                row_index,
                token_index,
                &id,
                &definition,
                true,
                rows,
                &mut count,
            )?;
            for marker in ["", "!"] {
                let hidden = format!(
                    "{marker}[ordinary\nlabel](\nhttps://example.invalid\n\"{token}\nordinary title\"\n)"
                );
                token_verdict(
                    row_index,
                    token_index,
                    &hidden,
                    "该工具包。",
                    false,
                    rows,
                    &mut count,
                )?;
                token_verdict(row_index, token_index, &id, &hidden, true, rows, &mut count)?;
                let shown = format!(
                    "{marker}[visible\n{token}](\nhttps://example.invalid\n\"ordinary\ntitle\"\n)"
                );
                token_verdict(
                    row_index,
                    token_index,
                    &shown,
                    &shown,
                    false,
                    rows,
                    &mut count,
                )?;
                token_verdict(
                    row_index,
                    token_index,
                    &shown,
                    "该工具包。",
                    true,
                    rows,
                    &mut count,
                )?;
                token_verdict(
                    row_index,
                    token_index,
                    &format!("{hidden}\n{token}"),
                    "该工具包。",
                    true,
                    rows,
                    &mut count,
                )?;
            }
        }
        verdict(
            "The toolkit is here.",
            "该工具包在此。",
            false,
            "K absent",
            rows,
            &mut count,
        )?;
    }
    let real: Vec<_> = authority
        .iter()
        .filter(|entry| entry.id.contains("standardized"))
        .collect();
    if real.is_empty() {
        return Err("NO REAL SPECIMEN: root default catalogue lacks 'standardized'".to_owned());
    }
    for entry in real {
        let keep = surviving_other_tokens(&entry.id, rows, None)?;
        verdict(
            &entry.id,
            &format!(
                "…但不存在标准化的拼写，因此这里描述的是 PurRDF 的写法。SPARQL 1.2 GRAPH CONSTRUCT {keep}"
            ),
            false,
            "real standardized spelling",
            rows,
            &mut count,
        )?;
    }
    for (id, text, refusal) in [
        (
            "Other engines already ship a form of it, but no standardized spelling exists.",
            "其他引擎已经提供了某种形式，但不存在标准化的拼写。",
            false,
        ),
        (
            "Provenance of the output processing step.",
            "输出处理步骤的溯源。",
            false,
        ),
        (
            "Determinism determines performance.",
            "确定性决定性能。",
            false,
        ),
        (
            "Streaming entails a cost.",
            "流式处理蕴含着一定开销。",
            false,
        ),
        ("The value is literally 42.", "该常量的字面值为 42。", false),
        ("Reference materials.", "参考资料集合见附录。", false),
        (
            "Research Object projections",
            "研究对象 (Research Object) 投影",
            false,
        ),
        (
            "Research Object projections",
            "研究对象（Research Object）投影",
            false,
        ),
        (
            "Research Object projections",
            "研究对象（Research Object，RO）投影",
            false,
        ),
        ("RDFLib compatibility.", "RDFLib 兼容层。", false),
        (
            "PurRDF is an RDF 1.2 toolkit in Rust with Python and WebAssembly bindings.",
            "PurRDF 是一个用 Rust 编写的 RDF 1.2 工具包，提供 Python 与 WebAssembly 绑定。",
            false,
        ),
        ("Research Object projections", "研究物件（RO）投影", true),
        ("Research Object projections", "研究对象（RO）投影", true),
        ("The RDF toolkit PurRDF.", "PurRDF 工具包。", true),
        ("Entailment of the statement.", "陈述所蕴含的东西。", true),
        ("The GMEOW ontology.", "吉猫协议本体。", true),
        (
            "Every IRI is absolute.",
            "每个国际化资源标识符都是绝对的。",
            true,
        ),
    ] {
        verdict(
            id,
            text,
            refusal,
            "retained concrete control",
            rows,
            &mut count,
        )?;
    }
    for (id, text, refusal) in [
        (
            "[ordinary](https://example.invalid/provenance)",
            "引文出处见脚注。",
            false,
        ),
        (
            "[ordinary](https://example.invalid/a(provenance\\)x))",
            "引文出处见脚注。",
            false,
        ),
        (
            "[ordinary](https://example.invalid/foo'provenance)",
            "引文出处见脚注。",
            false,
        ),
        (
            "![provenance](https://example.invalid)",
            "引文出处见脚注。",
            true,
        ),
        (
            "[provenance][ref]\n[ref]: https://example.invalid",
            "引文出处见脚注。",
            true,
        ),
        (
            "[ordinary][ref]\n[ref]: https://example.invalid/provenance \"provenance\"",
            "引文出处见脚注。",
            false,
        ),
        (
            "<https://example.invalid/provenance>",
            "引文出处见脚注。",
            true,
        ),
        (
            "[ordinary](https://example.invalid/RDF)",
            "该工具包。",
            false,
        ),
        ("RDF", "[ordinary](https://example.invalid/RDF)", true),
        (
            "![ordinary](https://example.invalid/RDF \"RDF\")",
            "该工具包。",
            false,
        ),
        (
            "RDF",
            "![ordinary](https://example.invalid/RDF \"RDF\")",
            true,
        ),
        (
            "![RDF](https://example.invalid)",
            "![RDF](https://example.invalid)",
            false,
        ),
        ("<https://example.invalid/RDF>", "该工具包。", true),
        (
            "<https://example.invalid/RDF>",
            "<https://example.invalid/RDF>",
            false,
        ),
        ("outside `purrdf-core`", "核心之外", false),
        ("outside `purrdf-core`", "核外扩展", true),
        ("pro[xx](hidden)venance", "引文出处見脚注。", false),
        ("RDF", "R[xx](hidden)DF", true),
        (
            "[ordinary](\nhttps://example.invalid/provenance\n)",
            "引文出处见脚注。",
            false,
        ),
        (
            "![ordinary](\nhttps://example.invalid/provenance\n)",
            "引文出处见脚注。",
            false,
        ),
        (
            "[ordinary](\nhttps://example.invalid/provenance\n)\nProvenance matters.",
            "引文出处见脚注。",
            true,
        ),
        (
            "[provenance\nlabel](\nhttps://example.invalid\n)",
            "引文出处见脚注。",
            true,
        ),
        (
            "[ordinary](\nhttps://example.invalid/RDF\n)",
            "该工具包。",
            false,
        ),
        ("RDF", "[ordinary](\nhttps://example.invalid/RDF\n)", true),
        (
            "![ordinary](\nhttps://example.invalid/RDF\n)",
            "该工具包。",
            false,
        ),
        ("RDF", "![ordinary](\nhttps://example.invalid/RDF\n)", true),
        ("RDF", "[visible\nRDF](\nhttps://example.invalid\n)", false),
        ("RDF", "![visible\nRDF](\nhttps://example.invalid\n)", false),
        (
            "The weather was fine today.",
            "[label](\nhttps://example.invalid/RDF\n\"` [hidden](provenance)\"\n)\n真实资料类型 `safe`",
            true,
        ),
        (
            "[ordinary](\nhttps://example.invalid/no-term\n\nProvenance)",
            "引文出处见脚注。",
            true,
        ),
        (
            "[ordinary `]`\nlabel](\nhttps://example.invalid/provenance\n)",
            "引文出处见脚注。",
            false,
        ),
        (
            "RDF",
            "[ordinary `]`\nlabel](\nhttps://example.invalid/RDF\n)",
            true,
        ),
        (
            "[ordinary](\nhttps://example.invalid/no-term\n\nRDF)",
            "该工具包。",
            true,
        ),
        (
            "RDF",
            "[ordinary](\nhttps://example.invalid/no-term\n\nRDF)",
            false,
        ),
    ] {
        verdict(
            id,
            text,
            refusal,
            "visible Markdown control",
            rows,
            &mut count,
        )?;
    }
    verdict(
        "blank node",
        "",
        false,
        "empty untranslated",
        rows,
        &mut count,
    )?;
    verdict(
        "blank node",
        "blank node",
        false,
        "English msgstr",
        rows,
        &mut count,
    )?;
    for prefix in ["#, fuzzy\n", "#~ "] {
        let body = if prefix == "#~ " {
            "#~ msgid \"blank node\"\n#~ msgstr \"空白节点\"".to_owned()
        } else {
            format!("{prefix}msgid \"blank node\"\nmsgstr \"空白节点\"")
        };
        for entry in po::parse(&body)? {
            if entry.active {
                return Err("inactive PO entry became active".to_owned());
            }
        }
        count += 1;
    }
    for (text, translated) in [
        ("PurRDF 是用 Rust 编写的 RDF 1.2 工具包。", true),
        ("We render entailment as 蕴涵.", false),
    ] {
        if is_translation(text) != translated {
            return Err(format!("file selection control failed: {text}"));
        }
        count += 1;
    }
    let fence = "普通中文。\n```text\n资料类型\n具名图\n```\n普通中文。";
    if !offences("multiline fence", None, fence, rows)?.is_empty() {
        return Err("global rule leaked into multiline fence".to_owned());
    }
    count += 1;
    Ok(count)
}

fn anchor_controls(rows: &[Row], count: &mut usize) -> Result<(), String> {
    for (row_index, row) in rows.iter().enumerate() {
        for (anchor, pattern) in row.anchors.iter().zip(&row.anchor_patterns) {
            let probes = if let Some(body) = regex_body(anchor) {
                match body {
                    r"literal(?!ly)" => vec![
                        ("literal".to_owned(), true),
                        ("literally".to_owned(), false),
                    ],
                    r"entail(?!s\b)" => vec![
                        ("entailment".to_owned(), true),
                        ("entails".to_owned(), false),
                        ("entails中".to_owned(), true),
                    ],
                    r"drop\w*\s+(?:\w+\s+){0,2}[_*]?loud" => vec![
                        ("drop it _loudly".to_owned(), true),
                        ("dropped中\u{85}row loudly".to_owned(), true),
                        ("drop quietly".to_owned(), false),
                    ],
                    r"lossy\b[^.]*loud" => vec![
                        ("lossy row loud".to_owned(), true),
                        ("lossy.row loud".to_owned(), false),
                        ("lossy中 loud".to_owned(), false),
                    ],
                    r"(?<![\w-])consistency(?![\w-])" => vec![
                        ("logical consistency here".to_owned(), true),
                        ("DL-consistency".to_owned(), false),
                        ("consistency-shaped".to_owned(), false),
                        ("consistency中".to_owned(), false),
                    ],
                    _ => {
                        return Err(format!(
                            "{}: regex anchor {anchor:?} needs explicit positive/negative compatibility specimens",
                            row.term
                        ));
                    }
                }
            } else {
                vec![(anchor.clone(), true), (format!("X{anchor}"), false)]
            };
            for (id, applies) in probes {
                if pattern.matches(&surface::project(&id).visible)? != applies {
                    return Err(format!(
                        "{}: anchor {anchor:?} fails compatibility specimen {id:?}",
                        row.term
                    ));
                }
                *count += 1;
                for rule in &row.rejected {
                    let keep = surviving_other_tokens(&id, rows, None)?;
                    let translated = format!("本页使用{}一词。{keep}", specimen(rule)?);
                    let found = probe(&id, &translated, rows)?;
                    if found
                        .iter()
                        .any(|offence| offence.row == row_index && offence.token.is_none())
                        != applies
                    {
                        return Err(format!(
                            "{}: anchor {anchor:?} fails production rejection specimen {id:?}",
                            row.term
                        ));
                    }
                    *count += 1;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_escapes_and_malformed_contract() {
        assert_eq!(
            split_cells(r"| a | `/x\s*\|y/` | `{| |}` |"),
            ["a", r"`/x\s*|y/`", "`{| |}`"]
        );
        for text in [
            "no table",
            "| # | Term | Anchor | Rendering | Basis | Rejected | Note |\n| - |\n",
            "| # | Term | Anchor | Rendering | Basis | Rejected | Note |\n| - | - | - | - | - | - | - |\n| 1 | term | term | | E | — | |",
        ] {
            assert!(parse_glossary(text, Path::new("fixture.md")).is_err());
        }
    }

    #[test]
    fn keep_english_requires_literal_anchors_and_non_keep_regex_remains_active() {
        let table = |anchor: &str, basis: &str| {
            format!(
                "| {} |\n| - | - | - | - | - | - | - |\n| 1 | literal | {anchor} | literal | {basis} | — | |\n",
                HEADER.join(" | ")
            )
        };
        for anchor in ["`/literal(?!ly)/`", "`literal`、`/other/`"] {
            let error = parse_glossary(&table(anchor, "K"), Path::new("fixture.md")).unwrap_err();
            assert!(error.contains("glossary:3 row \"literal\""));
            assert!(error.contains("K rows require literal anchors"));
        }
        let literal_rows =
            parse_glossary(&table("`literal`", "K"), Path::new("fixture.md")).unwrap();
        assert_eq!(
            offences("fixture.po:7", Some("literal"), "字面。", &literal_rows)
                .unwrap()
                .len(),
            1
        );
        assert!(
            offences("fixture.po:7", Some("literal"), "literal。", &literal_rows)
                .unwrap()
                .is_empty()
        );
        let regex_rows =
            parse_glossary(&table("`/literal(?!ly)/`", "E"), Path::new("fixture.md")).unwrap();
        assert!(regex_rows[0].anchored("literal").unwrap());
        assert!(!regex_rows[0].anchored("literally").unwrap());
    }

    #[test]
    fn selected_glossary_runs_all_default_controls() {
        let root = purrdf_testkit::paths::workspace_root();
        let rows =
            parse_glossary(&read(&root.join(GLOSSARY)).unwrap(), &root.join(GLOSSARY)).unwrap();
        consistency(&rows).unwrap();
        let authority = po::parse(&read(&root.join(CATALOGUE)).unwrap()).unwrap();
        assert!(self_test(&rows, &authority).unwrap() > 500);
        assert!(self_test(&rows, &[]).is_err());
    }

    #[test]
    fn whole_document_global_diagnostics_have_real_lines() {
        let root = purrdf_testkit::paths::workspace_root();
        let rows =
            parse_glossary(&read(&root.join(GLOSSARY)).unwrap(), &root.join(GLOSSARY)).unwrap();
        let found = offences(
            "translated.md",
            None,
            "中文\n```\n资料类型\n```\n普通资料类型。",
            &rows,
        )
        .unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].message.starts_with("translated.md:5:"));
    }
}
