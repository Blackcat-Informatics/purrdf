from pathlib import Path
from datetime import datetime, timezone
import hashlib
import json
import subprocess
import sys

worktree = Path("/home/paudley/Active/purrdf/.worktrees/387-blank-scope-investigation")
archive = Path("/home/paudley/Active/purrdf/.stage/387/scope-evidence/final/review-source")
archive.mkdir(parents=True, exist_ok=True)

def git(*args):
    return subprocess.check_output(["git", *args], cwd=worktree, text=True).strip()

head = git("rev-parse", "HEAD")
if git("status", "--porcelain"):
    raise SystemExit("refusing qualification of a dirty source tree")
base = git("rev-parse", "origin/main")
paths = git("diff", "--name-only", base + "...HEAD").splitlines()
files = {name: hashlib.sha256((worktree / name).read_bytes()).hexdigest()
         for name in paths if (worktree / name).is_file()}
manifest = {
    "captured_utc": datetime.now(timezone.utc).isoformat(),
    "head": head, "base": base, "files": files,
    "normal_commit_hooks": True, "qualification": "running",
    "gates": {}, "logs": {},
}
manifest_path = archive / "source-manifest.json"

def write_manifest():
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")

write_manifest()
commands = [
    (["make", "check"], "make-check.log"),
    (["make", "wasm"], "make-wasm.log"),
    (["make", "conformance", "CONFORMANCE_ARGS="], "make-conformance.log"),
    (["bash", "scripts/check-generated.sh"], "check-generated.log"),
]
for command, log_name in commands:
    if git("rev-parse", "HEAD") != head or git("status", "--porcelain"):
        raise SystemExit("source identity changed before qualification command")
    log_path = archive / log_name
    print("START", head, " ".join(command), flush=True)
    with log_path.open("wb") as log:
        result = subprocess.run(command, cwd=worktree, stdout=log, stderr=subprocess.STDOUT)
    manifest["gates"][" ".join(command)] = result.returncode
    manifest["logs"][log_name] = hashlib.sha256(log_path.read_bytes()).hexdigest()
    if result.returncode:
        manifest["qualification"] = "failed"
        write_manifest()
        print("FAIL", result.returncode, log_path, flush=True)
        raise SystemExit(result.returncode)
    print("PASS", log_path, flush=True)
    write_manifest()
if git("rev-parse", "HEAD") != head or git("status", "--porcelain"):
    manifest["qualification"] = "source_changed"
    write_manifest()
    raise SystemExit("source identity changed during qualification")
manifest["qualification"] = "passed"
manifest["qualified_utc"] = datetime.now(timezone.utc).isoformat()
write_manifest()
print("QUALIFIED", head, flush=True)
