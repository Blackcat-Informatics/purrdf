from pathlib import Path
import re, subprocess
root = Path(__file__).parents[2]
methods = set("""intern_iri intern_blank intern_literal intern_triple intern_owned_term intern_owned_term_scoped intern_text_blank intern_value intern_iri_value intern_blank_value intern_literal_value intern_triple_value push_quad push_quad_with_handle push_owned_quad push_owned_quad_scoped push_owned_reifier push_owned_reifier_scoped push_owned_annotation push_owned_annotation_scoped push_reifier push_reifier_in_graph push_annotation push_annotation_in_graph push_dataset next_quad_handle append_validated register_unit register_artifact intern_origin_set""".split())
pattern = re.compile(r"\.\s*(" + "|".join(sorted(methods)) + r")\s*\(")

def masked(source):
    result = list(source)
    n, i = len(source), 0
    while i < n:
        end = i
        if source.startswith("//", i):
            end = source.find("\n", i)
            if end < 0: end = n
        elif source.startswith("/*", i):
            depth, end = 1, i + 2
            while depth and end < n:
                if source.startswith("/*", end): depth += 1; end += 2
                elif source.startswith("*/", end): depth -= 1; end += 2
                else: end += 1
        elif source[i] == "r" and (m := re.match(r'r(#+)?"', source[i:])):
            hashes = m[1] or ""
            close = source.find('"' + hashes, i + len(m[0]))
            end = n if close < 0 else close + 1 + len(hashes)
        elif source[i] == '"':
            end = i + 1
            while end < n:
                if source[end] == "\\": end += 2
                elif source[end] == '"': end += 1; break
                else: end += 1
        elif source[i] == "'" and (m := re.match(r"'(?:\\.|[^\\'\n])'", source[i:])):
            end = i + len(m[0])
        if end > i:
            for j in range(i, end):
                if result[j] != "\n": result[j] = " "
            i = end
        else: i += 1
    return "".join(result)

def close(source, opening, left, right):
    depth = 1
    for i in range(opening + 1, len(source)):
        if source[i] == left: depth += 1
        elif source[i] == right:
            depth -= 1
            if not depth: return i + 1
    raise RuntimeError("unbalanced source")

files = subprocess.check_output(["rg", "--files", "crates", "-g", "*.rs"], cwd=root, text=True).splitlines()
changes = []
for path in files:
    if path.startswith("crates/datalog/"): continue
    file = root / path
    source = file.read_text()
    mask = masked(source)
    if any(part in Path(path).parts for part in ("tests", "benches", "examples")):
        spans = [(0, len(mask))]
    else:
        spans = []
        for cfg in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", mask):
            start = mask.find("{", cfg.end())
            if start >= 0: spans.append((start, close(mask, start, "{", "}")))
        if re.search(r"(?:^|/)tests(?:\.rs|/)|_tests\.rs$", path): spans = [(0, len(mask))]
    edits = []
    for match in pattern.finditer(mask):
        if not any(a <= match.start() < b for a, b in spans): continue
        end = close(mask, match.end() - 1, "(", ")")
        suffix = mask[end:].lstrip()
        if suffix.startswith("?") or re.match(r"\.\s*(?:unwrap|expect|map_err|is_err|is_ok|unwrap_err|unwrap_or_else|unwrap_or|ok|err)\b", suffix):
            continue
        edits.append((end, ".unwrap()"))
    for pos, text in reversed(edits):
        source = source[:pos] + text + source[pos:]
    if edits:
        file.write_text(source)
        changes.append((path, len(edits)))
print(f"Adjusted {sum(n for _, n in changes)} authored successful fixture calls in {len(changes)} files.")
for path, count in changes: print(f"{count}\t{path}")

