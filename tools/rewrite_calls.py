# Why not Rust: one-off scratch rewrite of call sites across many files; lives under the ignored .stage directory and is never committed.
import re, sys

RENAME = {
    "try_canonicalize_with": "try_canonicalize",
    "try_canonicalize_flat_view": "try_canonicalize_checked_view",
    "try_canonicalize_flat_graph_view": "try_canonicalize_checked_graph_view",
    "try_canonical_sha256_view": "try_canonical_digest_view",
    "try_canonical_blake3_view": "try_canonical_digest_view",
    "try_canonicalize_sha256_view": "try_canonicalize_digest_view",
    "try_flat_digest_view": "try_canonical_digest_checked_view",
    "try_flat_blake3_view": "try_canonical_digest_checked_view",
    "check_admissible_flat_view": "check_admissible_checked_view",
    "RDFC_V2_CORPUS_POLICY": "CANON_CORPUS_POLICY",
}
DROP_HASH_IN = {
    "try_canonicalize_with", "try_canonicalize_view", "try_canonicalize_graph_view",
    "try_canonicalize_flat_view", "try_canonicalize_flat_graph_view", "try_canonical_sha256_view",
    "try_canonical_blake3_view", "try_canonicalize_sha256_view", "try_flat_digest_view",
    "try_flat_blake3_view",
}
HASH_ARG = re.compile(r"^(?:[A-Za-z_][A-Za-z0-9_]*::)*CanonHash::(?:Sha256|Sha384)$")
CALL = re.compile(r"\b(" + "|".join(sorted(DROP_HASH_IN, key=len, reverse=True)) + r")\s*\(")

def split_args(text):
    args, depth, cur, i = [], 0, [], 0
    in_str = False
    while i < len(text):
        c = text[i]
        if in_str:
            cur.append(c)
            if c == "\\": cur.append(text[i+1]); i += 1
            elif c == '"': in_str = False
        elif c == '"':
            in_str = True; cur.append(c)
        elif c in "([{<" and not (c == "<" and False):
            depth += 1; cur.append(c)
        elif c in ")]}":
            depth -= 1; cur.append(c)
        elif c == "," and depth == 0:
            args.append("".join(cur)); cur = []
        else:
            cur.append(c)
        i += 1
    args.append("".join(cur))
    return args

def rewrite(src):
    out, pos = [], 0
    while True:
        m = CALL.search(src, pos)
        if not m: break
        # skip definitions / paths like `fn name(`
        before = src[max(0, m.start()-4):m.start()]
        start = m.end()
        depth, i, in_str = 1, start, False
        while i < len(src) and depth:
            c = src[i]
            if in_str:
                if c == "\\": i += 1
                elif c == '"': in_str = False
            elif c == '"': in_str = True
            elif c in "([{": depth += 1
            elif c in ")]}": depth -= 1
            i += 1
        body = src[start:i-1]
        if before.endswith("fn "):
            out.append(src[pos:i]); pos = i; continue
        args = split_args(body)
        kept = [a for a in args if not HASH_ARG.match(a.strip())]
        if len(kept) != len(args):
            # keep trailing-comma style of the original
            trailing = body.rstrip().endswith(",") and body.strip() != ""
            new_body = ",".join(a for a in kept if a.strip() != "" or False)
            if trailing and not new_body.rstrip().endswith(","):
                new_body += ","
            body = new_body
        out.append(src[pos:start]); out.append(body); out.append(")")
        pos = i
    out.append(src[pos:])
    return "".join(out)

def rename(src):
    for old, new in RENAME.items():
        src = re.sub(r"\b" + old + r"\b", new, src)
    return src

def drop_canonhash_import(src):
    if re.search(r"CanonHash::", src): return src
    src = re.sub(r"\bCanonHash,\s*", "", src)
    src = re.sub(r",\s*CanonHash\b(?!::)", "", src)
    return src

for path in sys.argv[1:]:
    s = open(path).read()
    t = rename(rewrite(s))
    t = drop_canonhash_import(t)
    if t != s:
        open(path, "w").write(t)
        print("rewrote", path)
