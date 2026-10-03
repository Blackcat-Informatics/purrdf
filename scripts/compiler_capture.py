# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Shared source and held-slot authority for compiler artifact capture."""

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

def source_identity(root: Path) -> str:
    """Pin checked-in compiler/header inputs, without depending on Git state."""
    inputs = [root / name for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]]
    for directory in [root / "crates", root / "bindings", root / ".cargo"]:
        if directory.is_dir():
            inputs.extend(path for path in directory.rglob("*")
                          if path.is_file() and path.suffix in {".rs", ".toml"})
    digest = hashlib.sha256()
    for path in sorted(set(inputs)):
        if not path.is_file():
            continue
        name = path.relative_to(root).as_posix().encode()
        content = path.read_bytes()
        for part in [name, content]:
            digest.update(len(part).to_bytes(8, "little"))
            digest.update(part)
    return digest.hexdigest()



def stage_capabilities() -> dict | None:
    """Discover Stage's documented slot layout; ordinary Cargo returns nonzero."""
    result = subprocess.run(["cargo", "--stage-asm-capabilities"],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, check=False)
    if result.returncode:
        return None
    value = json.loads(result.stdout)
    if (set(value) != {"version", "slots", "locks"} or value["version"] != 1
            or not all(isinstance(value[key], str) and Path(value[key]).is_absolute()
                       for key in ["slots", "locks"])):
        raise ValueError("unsupported Stage slot capability record")
    return value



def slot_lock(artifact: Path, capabilities: dict) -> Path:
    """Resolve the exact supported slot lock for one registered artifact."""
    relative = artifact.resolve().relative_to(Path(capabilities["slots"]).resolve())
    if (len(relative.parts) < 4 or len(relative.parts[0]) != 16
            or any(c not in "0123456789abcdef" for c in relative.parts[0])
            or not relative.parts[1].isdigit() or not 0 <= int(relative.parts[1]) < 32
            or relative.parts[2] != "target"):
        raise ValueError("compiler artifact is outside a supported Stage slot")
    return (Path(capabilities["locks"]) / relative.parts[0] / f"{relative.parts[1]}.lock").resolve()


def held_slot(artifact: Path, capabilities: dict) -> Path:
    """Verify the reported artifact's exact Stage slot remains exclusively leased."""
    import fcntl  # Stage runs on Linux; the ordinary Cargo path needs no flock.
    lock = slot_lock(artifact, capabilities)
    with lock.open("rb") as lease:
        try:
            fcntl.flock(lease.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return lock.resolve()
        fcntl.flock(lease.fileno(), fcntl.LOCK_UN)
    raise ValueError("compiler capture requires its held Stage slot lease")



def real_cargo() -> Path:
    """Resolve the documented delegate producer without invoking another wrapper."""
    real = Path(os.environ.get("CARGO_REAL") or
                str(Path(os.environ.get("RUSTUP_HOME", str(Path.home() / "stage/packages/rustup")))
                    / "active-toolchain/bin/cargo")).resolve()
    wrapper = shutil.which("cargo")
    if not os.access(real, os.X_OK) or (wrapper and real == Path(wrapper).resolve()):
        raise ValueError("Stage's documented real Cargo executable is unavailable or recursive")
    return real
