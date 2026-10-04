#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Generate reproducible recipient notices, and audit the actual release archives.

Source packages carry notices for their local inherited material. Linked Python,
WASM and C artifacts additionally carry the resolved normal/build dependency
closure across targets. This deliberately includes platform/build inputs that
may not contribute machine code; the inventory states that scope, rather than
claiming every input occurs in every binary. No dependency is relicensed.
"""

from __future__ import annotations

import argparse
import email.parser
import fnmatch
import gzip
import hashlib
import importlib.util
import io
import json
import os
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath
from types import SimpleNamespace
from unittest.mock import patch

import tomllib

ROOT = Path(__file__).resolve().parent.parent
FIRST_PARTY = "MIT OR Apache-2.0 OR MulanPSL-2.0"
FIRST_TEXTS = ("LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN")
LEGAL_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "AUTHORS")
IGNORED_PARTS = {"tests", "examples", "benches", "testdata", ".github"}
COMPILER_ENV = (
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
)


def native_launcher(command: str) -> None:
    executable = shutil.which(command)
    if not executable:
        raise ValueError(f"Docs compiler capture requires {command}")
    with Path(executable).resolve().open("rb") as binary:
        magic = binary.read(4)
    if magic not in (
        b"\x7fELF",
        b"\xcf\xfa\xed\xfe",
        b"\xfe\xed\xfa\xcf",
    ) and not magic.startswith(b"MZ"):
        raise ValueError(
            f"Docs compiler capture refuses an unverified {command} launch script"
        )


def compiler_identity() -> bytes:
    """Refuse compiler redirection before identifying a Docs Cargo build.

    The assembly command supplies no Cargo --config overrides. Inspect only
    compiler-selection inputs; no environment or unrelated configuration is
    printed. Arbitrary launch scripts and Cargo configuration includes cannot
    be certified by a later standalone rustc stamp, so this path refuses them.
    """
    if any(os.environ.get(name) for name in COMPILER_ENV):
        raise ValueError(
            "Docs compiler capture refuses compiler/wrapper environment overrides"
        )
    for command in ("cargo", "rustc"):
        native_launcher(command)
    directories = [ROOT, *ROOT.parents]
    config_dirs = [directory / ".cargo" for directory in directories]
    config_dirs.append(Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")))
    for directory in config_dirs:
        for name in ("config", "config.toml"):
            path = directory / name
            if not path.exists():
                continue
            config = tomllib.loads(path.read_text())
            build = config.get("build", {})
            environment = config.get("env", {})
            if (
                "include" in config
                or build.get("rustc")
                or any(
                    build.get(key) and os.environ.get(variable) != ""
                    for key, variable in (
                        ("rustc-wrapper", "RUSTC_WRAPPER"),
                        ("rustc-workspace-wrapper", "RUSTC_WORKSPACE_WRAPPER"),
                    )
                )
                or any(environment.get(key) for key in COMPILER_ENV)
            ):
                raise ValueError(
                    "Docs compiler capture refuses Cargo compiler redirection"
                )
    result = subprocess.run(["rustc", "-vV"], capture_output=True, check=True)
    if not result.stdout.startswith(b"rustc ") or b"commit-hash: " not in result.stdout:
        raise ValueError("Docs compiler capture received no identified Rust compiler")
    return result.stdout


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def python_backend():
    location = ROOT / "bindings/python/purrdf_build_backend.py"
    spec = importlib.util.spec_from_file_location("purrdf_build_backend", location)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def add_runtime(path: Path, profile: str) -> None:
    contents = archive_files(path)
    roots = [
        name.removesuffix("inventory.json")
        for name, value in contents.items()
        if name.endswith("/licenses/inventory.json")
        and json.loads(value).get("profile") == profile
    ]
    if not roots:
        raise ValueError("runtime capture requires an existing recipient notice bundle")
    runtime = python_backend().runtime_bundle()
    for prefix in roots:
        contents.update(
            {f"{prefix}runtime/{name}": value for name, value in runtime.items()}
        )
    with tarfile.open(path) as archive:
        infos = archive.getmembers()
    existing = {item.name for item in infos}
    for name in sorted(contents.keys() - existing):
        info = tarfile.TarInfo(name)
        info.mode = 0o644
        infos.append(info)
    with tempfile.NamedTemporaryFile(
        dir=path.parent, suffix=".tar.gz", delete=False
    ) as sink:
        temporary = Path(sink.name)
    try:
        with (
            temporary.open("wb") as sink,
            gzip.GzipFile(fileobj=sink, filename="", mode="wb", mtime=0) as compressed,
            tarfile.open(fileobj=compressed, mode="w") as archive,
        ):
            for info in infos:
                if info.isfile():
                    value = contents[info.name]
                    info.size = len(value)
                    archive.addfile(info, io.BytesIO(value))
                else:
                    archive.addfile(info)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--offline"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def inventory() -> list[dict]:
    entries = tomllib.loads((ROOT / "license-inventory.toml").read_text())["material"]
    names = [entry["id"] for entry in entries]
    if len(names) != len(set(names)):
        raise ValueError("duplicate inherited-material identifier")
    for entry in entries:
        if entry.get("separate_archive") not in (None, "purrdf-text-lexicons"):
            raise ValueError(f"{entry['id']}: unknown separate data archive")
        if entry.get("separate_archive") and any(
            not path.startswith("crates/text/lexicons/")
            for path in entry.get("paths", [])
        ):
            raise ValueError(f"{entry['id']}: separate lexicon scope covers other material")
        for key in (
            "id",
            "license",
            "source",
            "scope",
            "paths",
            "notices",
            "provenance",
        ):
            if not entry.get(key):
                raise ValueError(f"{entry.get('id')}: missing {key}")
        for path in [*entry["notices"], *entry["provenance"]]:
            if not (ROOT / path).is_file():
                raise ValueError(f"{entry['id']}: missing evidence {path}")
        for pattern in entry["paths"]:
            if next(ROOT.glob(pattern), None) is None:
                raise ValueError(f"{entry['id']}: stale material path {pattern}")
    return entries


def legal_files(directory: Path) -> list[Path]:
    return sorted(
        path
        for path in directory.rglob("*")
        if path.is_file()
        and path.name.upper().startswith(LEGAL_PREFIXES)
        and not (set(path.relative_to(directory).parts) & IGNORED_PARTS)
    )


def closure(start: str, nodes: dict[str, dict]) -> set[str]:
    visited: set[str] = set()
    pending = [start]
    while pending:
        identifier = pending.pop()
        if identifier in visited:
            continue
        visited.add(identifier)
        for edge in nodes[identifier]["deps"]:
            if any(kind["kind"] != "dev" for kind in edge["dep_kinds"]):
                pending.append(edge["pkg"])
    return visited


def profiles(meta: dict) -> dict[str, tuple[Path, set[str], bool]]:
    packages = {package["id"]: package for package in meta["packages"]}
    members = {
        identifier: packages[identifier] for identifier in meta["workspace_members"]
    }
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
    result = {
        package["name"]: (Path(package["manifest_path"]).parent, {identifier}, False)
        for identifier, package in members.items()
        if package.get("publish") != []
    }
    for name, crate, location in (
        ("python", "purrdf-python", "bindings/python"),
        ("npm", "purrdf-wasm", "crates/rdf-wasm/js"),
        ("c", "purrdf-capi", "crates/rdf-capi"),
    ):
        identifier = next(
            key for key, value in members.items() if value["name"] == crate
        )
        result[name] = (ROOT / location, closure(identifier, nodes), True)
    result["python-rdflib"] = (ROOT / "bindings/python-rdflib-shadow", set(), False)
    return dict(sorted(result.items()))


def bundle(
    profile: str,
    details: tuple[Path, set[str], bool],
    meta: dict,
    inherited: list[dict],
) -> dict[str, bytes]:
    _directory, identifiers, linked = details
    packages = {package["id"]: package for package in meta["packages"]}
    files = {name: (ROOT / name).read_bytes() for name in FIRST_TEXTS}
    files["CC-BY-4.0.txt"] = (ROOT / "LICENSES/CC-BY-4.0.txt").read_bytes()
    components = []
    source_roots = [
        Path(packages[key]["manifest_path"]).parent
        for key in identifiers
        if packages[key]["source"] is None
    ]
    # Maturin's source archive carries workspace sources and test fixtures.
    # Preserve their notices even though the compiled extension excludes them.
    if profile == "python":
        source_roots = [ROOT]
    for entry in inherited:
        # These data are excluded from Cargo packages and distributed in their
        # own fully noticed archive. Do not attach their terms to library code
        # merely because their source checkout sits beneath a crate directory.
        if entry.get("separate_archive"):
            continue
        if not any(
            path == base or base in path.parents
            for pattern in entry["paths"]
            for path in [ROOT / pattern.split("*")[0].rstrip("/")]
            for base in source_roots
        ):
            continue
        component = {
            key: entry[key]
            for key in ("id", "license", "source", "scope", "paths", "provenance")
        }
        component["notices"] = []
        for index, rel in enumerate(entry["notices"]):
            target = f"third-party/{entry['id']}/{index}-{Path(rel).name}"
            files[target] = (ROOT / rel).read_bytes()
            component["notices"].append(target)
        components.append(component)
    if linked:
        locked = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
        for identifier in sorted(identifiers):
            package = packages[identifier]
            if package["source"] is None:
                continue
            if not package.get("license"):
                raise ValueError(f"{profile}: no declared license for {identifier}")
            path = Path(package["manifest_path"]).parent
            notices = legal_files(path)
            if not notices:
                raise ValueError(
                    f"{profile}: no license/notice text in resolved input {identifier}"
                )
            name = f"{package['name']}-{package['version']}"
            component = {
                "id": name,
                "license": package["license"],
                "source": package["source"],
                "repository": package.get("repository"),
                "scope": "resolved normal/build input, including target-specific alternatives",
                "notices": [],
            }
            records = [
                item
                for item in locked
                if item["name"] == package["name"]
                and item["version"] == package["version"]
                and item.get("source") == package["source"]
            ]
            if len(records) != 1 or not records[0].get("checksum"):
                raise ValueError(
                    f"{profile}: missing locked source checksum for {identifier}"
                )
            component["source_checksum"] = records[0]["checksum"]
            for notice in notices:
                target = f"dependencies/{name}/{notice.relative_to(path).as_posix()}"
                files[target] = notice.read_bytes()
                component["notices"].append(target)
            components.append(component)
    components.sort(key=lambda item: item["id"])
    notice = [
        "PurRDF recipient notices",
        "========================",
        "",
        "Copyright 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>",
        f"First-party code: {FIRST_PARTY} (choose any one option).",
        "The three complete first-party license texts accompany this notice.",
        "Third-party and derived material retains its own terms, in addition to",
        "the chosen first-party license. Documentation with CC-BY-4.0 headers",
        "retains that grant. No trademark rights are granted.",
        "",
        "This inventory preserves notices for the source material and resolved",
        "build inputs described below. Platform alternatives and build inputs",
        "need not occur in every compiled binary. Test data is source-only.",
        "",
    ]
    for item in components:
        notice.extend(
            [
                f"{item['id']}: {item['license']}",
                f"Scope: {item['scope']}",
                f"Source: {item['source']}",
            ]
        )
        notice.extend(f"Evidence: {value}" for value in item.get("provenance", []))
        notice.extend(f"Notice: {value}" for value in item["notices"])
        notice.append("")
    files["NOTICE.txt"] = ("\n".join(notice) + "\n").encode()
    data = {
        "schema": 1,
        "profile": profile,
        "first_party_license": FIRST_PARTY,
        "components": components,
        "files": {name: digest(value) for name, value in sorted(files.items())},
    }
    if profile.startswith("purrdf"):
        data["package_license"] = next(
            package["license"]
            for package in packages.values()
            if package["name"] == profile
        )
    files["inventory.json"] = (
        json.dumps(data, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def materialize(
    destination: Path, expected: dict[str, bytes], write: bool
) -> list[str]:
    problems = []
    actual = {
        path.relative_to(destination).as_posix()
        for path in destination.rglob("*")
        if path.is_file()
    }
    for stale in sorted(actual - expected.keys()):
        if write:
            (destination / stale).unlink()
        else:
            problems.append(f"unexpected generated notice {destination / stale}")
    for name, value in sorted(expected.items()):
        path = destination / name
        if path.exists() and path.read_bytes() == value:
            continue
        if write:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(value)
        else:
            problems.append(f"missing/stale generated notice {path}")
    return problems


def archive_files(path: Path) -> dict[str, bytes]:
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            names = [item.filename for item in archive.infolist()]
            if len(names) != len(set(names)):
                raise ValueError(f"{path}: duplicate archive members")
            result = {
                item.filename: archive.read(item)
                for item in archive.infolist()
                if not item.is_dir()
            }
    else:
        with tarfile.open(path, "r:*") as archive:
            result = {}
            for member in archive.getmembers():
                if member.issym() or member.islnk():
                    raise ValueError(
                        f"{path}: archive symlink is not license evidence: {member.name}"
                    )
                if member.isfile():
                    if member.name in result:
                        raise ValueError(
                            f"{path}: duplicate archive member {member.name}"
                        )
                    stream = archive.extractfile(member)
                    assert stream is not None
                    result[member.name] = stream.read()
    return validate_members(path, result)


def validate_members(path: Path, result: dict[str, bytes]) -> dict[str, bytes]:
    for name in result:
        pure = PurePosixPath(name)
        if pure.is_absolute() or ".." in pure.parts:
            raise ValueError(f"{path}: unsafe archive member {name}")
        if set(pure.parts) & {
            ".stage",
            ".codex",
            ".goals",
            ".baseline",
            ".agents",
            ".worktrees",
            ".git",
        } or pure.name.startswith("ROADMAP-"):
            raise ValueError(f"{path}: private working material entered the package")
        if "xmlconf" in pure.parts and any(
            part in {"xmltest", "eduni", "sun", "ibm", "oasis", "japanese"}
            for part in pure.parts
        ):
            raise ValueError(
                f"{path}: acquired XML conformance payload may not be redistributed"
            )
        if "lexicons" in pure.parts:
            raise ValueError(
                f"{path}: separately distributed lexicon data entered a library package"
            )
    return result


def directory_files(path: Path) -> dict[str, bytes]:
    if path.is_symlink() or not path.is_dir():
        raise ValueError(f"{path}: recipient directory must be a real directory")
    result = {}
    for entry in sorted(path.rglob("*")):
        mode = entry.lstat().st_mode
        if stat.S_ISLNK(mode):
            raise ValueError(
                f"{path}: directory symlink is not license evidence: {entry}"
            )
        if stat.S_ISREG(mode):
            result[entry.relative_to(path).as_posix()] = entry.read_bytes()
        elif not stat.S_ISDIR(mode):
            raise ValueError(f"{path}: non-regular recipient payload: {entry}")
    return validate_members(path, result)


def audit(path: Path, profile: str, expected: dict[str, bytes]) -> dict:
    result = audit_payload(path, profile, expected, archive_files(path))
    return {**result, "sha256": digest(path.read_bytes())}


def audit_payload(
    path: Path,
    profile: str,
    expected: dict[str, bytes],
    contents: dict[str, bytes],
    recipient_root: str = "package",
) -> dict:
    problems = []
    bundle_roots = []
    for member, value in contents.items():
        if not member.endswith("/licenses/inventory.json"):
            continue
        try:
            declared = json.loads(value)
        except (ValueError, UnicodeDecodeError):
            continue
        if declared.get("profile") == profile:
            bundle_roots.append(member.removesuffix("inventory.json"))
    if not bundle_roots:
        raise ValueError(f"{path}: missing packaged inventory for {profile}")
    for prefix in bundle_roots:
        for name, value in sorted(expected.items()):
            actual = contents.get(prefix + name)
            if actual is None:
                problems.append(f"missing {prefix}{name}")
            elif actual != value:
                problems.append(f"changed license evidence {prefix}{name}")
    runtime_required = (
        profile == "c"
        or profile == "python"
        and zipfile.is_zipfile(path)
        or profile == "npm"
        and any(name.endswith(".wasm") for name in contents)
    )
    runtime = python_backend().runtime_bundle() if runtime_required else {}
    for prefix in bundle_roots:
        for name, value in runtime.items():
            if contents.get(f"{prefix}runtime/{name}") != value:
                problems.append(
                    f"missing/stale linked runtime notice {prefix}runtime/{name}"
                )
    if problems:
        raise ValueError(f"{path}: " + "; ".join(problems))
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"][
        "version"
    ]
    if profile.startswith("purrdf") and profile != "python-rdflib":
        manifests = [
            value
            for name, value in contents.items()
            if name == f"{profile}-{version}/Cargo.toml"
        ]
        if len(manifests) != 1:
            raise ValueError(
                f"{path}: missing Cargo manifest at the exact release identity"
            )
        package = tomllib.loads(manifests[0].decode())["package"]
        if package["name"] != profile or package["version"] != version:
            raise ValueError(f"{path}: wrong Cargo package name/version")
        if (
            package.get("license")
            != json.loads(expected["inventory.json"])["package_license"]
        ):
            raise ValueError(f"{path}: wrong Cargo package license expression")
    elif profile in ("python", "python-rdflib"):
        project = "purrdf" if profile == "python" else "purrdf-rdflib"
        if not zipfile.is_zipfile(path):
            allowed_roots = {
                "PKG-INFO",
                "Cargo.lock",
                "Cargo.toml",
                "README.md",
                "pyproject.toml",
                "rust-toolchain.toml",
                ".gitignore",
            }
            if profile == "python":
                allowed_roots.add("purrdf_build_backend.py")
            unexpected = [
                name
                for name in contents
                if len(PurePosixPath(name).parts) == 2
                and PurePosixPath(name).name not in allowed_roots
            ]
            if unexpected:
                raise ValueError(f"{path}: unlisted Python source-archive root payload")
        candidates = [
            value
            for name, value in contents.items()
            if name.endswith(".dist-info/METADATA")
            or name.count("/") == 1
            and name.endswith("/PKG-INFO")
        ]
        if len(candidates) != 1:
            raise ValueError(f"{path}: missing unique Python core metadata")
        declared = email.parser.BytesParser().parsebytes(candidates[0])
        if declared["Name"] != project or declared["Version"] != version:
            raise ValueError(f"{path}: wrong Python package name/version")
        if runtime_required:
            declared_files = set(declared.get_all("License-File", []))
            for prefix in bundle_roots:
                license_root = prefix.split(".dist-info/licenses/", 1)[1]
                if any(
                    f"{license_root}runtime/{name}" not in declared_files
                    for name in runtime
                ):
                    raise ValueError(
                        f"{path}: linked runtime notices absent from Python License-File metadata"
                    )
        if profile == "python":
            location = ROOT / "bindings/python/purrdf_build_backend.py"
            module = python_backend()
            expression = module.expected(path, contents)
            if not zipfile.is_zipfile(path):
                dynamic = {
                    field.casefold() for field in declared.get_all("Dynamic", [])
                }
                if not {"license-expression", "license-file"}.issubset(dynamic):
                    raise ValueError(
                        f"{path}: native source License-Expression and License-File must be dynamic for wheel builds"
                    )
                helper = [
                    value
                    for name, value in contents.items()
                    if name.endswith("/bindings/python/purrdf_build_backend.py")
                ]
                configs = [
                    value
                    for name, value in contents.items()
                    if name.endswith("/bindings/python/pyproject.toml")
                ]
                consumer_helpers = [
                    value
                    for name, value in contents.items()
                    if name.count("/") == 1
                    and name.endswith("/purrdf_build_backend.py")
                ]
                consumer_configs = [
                    value
                    for name, value in contents.items()
                    if name.count("/") == 1 and name.endswith("/pyproject.toml")
                ]
                # Maturin relocates the binding pyproject to the sdist root and
                # rewrites its manifest/python-source paths for that layout.
                if (
                    len(helper) != 1
                    or helper[0] != location.read_bytes()
                    or consumer_helpers != [location.read_bytes()]
                    or len(consumer_configs) != 1
                ):
                    raise ValueError(f"{path}: missing or stale consumer build backend")
                for value in [*configs, *consumer_configs]:
                    config = tomllib.loads(value.decode())
                    if (
                        config["build-system"].get("build-backend")
                        != "purrdf_build_backend"
                        or config["build-system"].get("backend-path") != ["."]
                        or "license" not in config["project"].get("dynamic", [])
                    ):
                        raise ValueError(
                            f"{path}: source consumer backend does not qualify its wheel license"
                        )
        else:
            expression = f"({FIRST_PARTY}) AND CC-BY-4.0"
        if declared["License-Expression"] != expression:
            raise ValueError(f"{path}: wrong Python distribution license expression")
    elif profile == "npm":
        metadata_path = f"{recipient_root}/package.json"
        if metadata_path not in contents:
            raise ValueError(f"{path}: missing npm recipient package metadata")
        declared = json.loads(contents[metadata_path])
        if (
            declared["name"] != "@blackcatinformatics/purrdf"
            or declared["version"] != version
        ):
            raise ValueError(f"{path}: wrong npm package name/version")
    elif profile == "c":
        if not any(
            prefix.endswith("/share/purrdf/licenses/") for prefix in bundle_roots
        ):
            raise ValueError(f"{path}: missing installed C notice bundle")
        headers = [
            value
            for name, value in contents.items()
            if name.endswith("/include/purrdf/purrdf.h")
        ]
        if (
            len(headers) != 1
            or headers[0] != (ROOT / "crates/rdf-capi/include/purrdf.h").read_bytes()
        ):
            raise ValueError(f"{path}: missing or stale C ABI header")
        pkg_configs = [
            value.decode()
            for name, value in contents.items()
            if name.endswith("/pkgconfig/purrdf.pc")
        ]
        if (
            len(pkg_configs) != 1
            or f"Version: {version}" not in pkg_configs[0].splitlines()
        ):
            raise ValueError(f"{path}: missing or stale C package metadata")
        if not any(
            "/lib/libpurrdf" in name and value for name, value in contents.items()
        ):
            raise ValueError(f"{path}: missing C library payload")
    return {
        "artifact": path.name,
        "profile": profile,
        "notice_files": len(expected),
        "runtime_notice_files": len(runtime),
        "status": "passed",
    }


def audit_directory(
    path: Path, recipient_root: str, expected: dict[str, bytes]
) -> dict:
    relative = PurePosixPath(recipient_root)
    if (
        not relative.parts
        or relative.is_absolute()
        or ".." in relative.parts
        or relative.as_posix() != recipient_root
    ):
        raise ValueError("directory recipient root must be a canonical relative path")
    contents = directory_files(path)
    prefix = recipient_root + "/"
    npm = ROOT / "crates/rdf-wasm/js"
    code = ["package.json", "index.mjs"] + [
        "pkg/" + name
        for name in (
            "purrdf_jspi.mjs",
            "purrdf_wasm.js",
            "purrdf_wasm.d.ts",
            "purrdf_wasm_bg.wasm",
            "purrdf_wasm_bg.wasm.d.ts",
        )
    ]
    payload_hashes = {}
    for name in code:
        source = (npm / name).read_bytes()
        if contents.get(prefix + name) != source:
            raise ValueError(
                f"{path}: missing/stale assembled module input {prefix}{name}"
            )
        payload_hashes[name] = digest(source)
    shell_root = relative.parent
    shell = ROOT / "docs/playground"
    for name in (
        "index.html",
        "app.mjs",
        "engine.worker.mjs",
        "sarif.mjs",
        "style.css",
        "sw.mjs",
        "manifest.webmanifest",
        "examples/gallery.mjs",
    ):
        member = (shell_root / name).as_posix()
        source = (shell / name).read_bytes()
        if contents.get(member) != source:
            raise ValueError(f"{path}: missing/stale assembled shell input {member}")
        payload_hashes[member] = digest(source)
    wasm = contents[prefix + "pkg/purrdf_wasm_bg.wasm"]
    if not wasm.startswith(b"\x00asm\x01\x00\x00\x00"):
        raise ValueError(f"{path}: assembled module is not WebAssembly")
    compiler = contents.get(prefix + "build-compiler.txt")
    runtime = python_backend().runtime_bundle()
    actual_compiler = json.loads(runtime["inventory.json"])["compiler"].encode()
    if compiler != actual_compiler:
        raise ValueError(f"{path}: assembled module/compiler notice identity differs")
    receipt = audit_payload(path, "npm", expected, contents, recipient_root)
    hashes = {name: digest(value) for name, value in sorted(contents.items())}
    return {
        **receipt,
        "artifact_type": "assembled-directory",
        "recipient_root": recipient_root,
        "sha256": digest(json.dumps(hashes, sort_keys=True).encode()),
        "payload_files": len(contents),
        "payload_hashes": hashes,
        "module_input_hashes": payload_hashes,
        "compiler": actual_compiler.decode(),
        "compiler_record_sha256": digest(compiler),
    }


def docs_bundle(path: Path) -> dict[str, bytes]:
    """Docs-only asset grants; release profiles do not load these resources."""
    resources = ROOT / "docs/book/recipient-notices"
    resource_contents = directory_files(resources)
    specification = resource_contents.get("manifest.json")
    if specification is None:
        raise ValueError("missing Docs asset grant specification")
    declared = json.loads(specification)
    if declared.get("schema") != 1 or declared.get("mdbook_version") != "0.5.3":
        raise ValueError("unsupported Docs asset grant specification")
    files = {}
    for name, sha256 in declared["files"].items():
        validate_members(resources, {name: b""})
        if name not in resource_contents:
            raise ValueError("Docs recipient evidence must be a regular source file")
        value = resource_contents[name]
        if digest(value) != sha256:
            raise ValueError(f"changed pinned Docs grant/source form {name}")
        files[name] = value
    contents = directory_files(path)
    assets = {}
    resource_names = {}
    for item in declared["assets"]:
        if "name" in item:
            resource_names[re.sub(r"-[a-f0-9]{8}(?=\.)", "", item["name"])] = item[
                "name"
            ]
    for item in declared["assets"]:
        pattern = item.get("name", item.get("pattern"))
        for locale in item["locales"]:
            prefix = locale + "/" if locale else ""
            matches = [
                name
                for name in contents
                if name.startswith(prefix)
                and fnmatch.fnmatchcase(name.removeprefix(prefix), pattern)
            ]
            if len(matches) != 1:
                raise ValueError(f"Docs asset missing or duplicated: {prefix}{pattern}")
            name = matches[0]
            value = contents[name]
            if item.get("generated_source"):
                if (
                    not re.fullmatch(
                        r"(?:zh-Hans/)?(?:searchindex|toc)-[a-f0-9]{8}\.js", name
                    )
                    or digest(value)[:8] not in name
                ):
                    raise ValueError(f"invalid generated Docs source identity {name}")
                resource_names[
                    re.sub(r"-[a-f0-9]{8}(?=\.)", "", name.removeprefix(prefix))
                ] = name.removeprefix(prefix)
            elif not item.get("template") and digest(value) != item["source_sha256"]:
                raise ValueError(f"changed pinned Docs asset {name}")
            assets[name] = {
                "sha256": digest(value),
                "licenses": item["licenses"],
                "source": item["source"],
            }
    for item in declared["assets"]:
        if not item.get("template"):
            continue
        for locale in item["locales"]:
            prefix = locale + "/" if locale else ""
            source = files[item["source_form"]].decode()

            resource_prefix = "../" if item["name"].startswith("fonts/") else ""

            def expand(match, resource_prefix=resource_prefix):
                name = match.group(1)
                if name not in resource_names:
                    raise ValueError(f"unbound Docs resource expansion {name}")
                return resource_prefix + resource_names[name]

            value = re.sub(r'\{\{ resource "([^"]+)" \}\}', expand, source).encode()
            if contents[prefix + item["name"]] != value:
                raise ValueError(
                    f"changed generated Docs source expansion {prefix}{item['name']}"
                )
    for name, value in contents.items():
        if name.startswith(("playground/", "recipient-notices/")):
            continue
        if (
            Path(name).suffix in {".js", ".css", ".woff2", ".woff"}
            and name not in assets
        ):
            raise ValueError(f"uninventoried Docs frontend asset {name}")
        if name.endswith(".html"):
            assets[name] = {
                "sha256": digest(value),
                "licenses": ["MPL-2.0", "CC-BY-4.0"],
                "source": declared["mdbook_source"],
            }
    if not {"index.html", "zh-Hans/index.html"}.issubset(contents):
        raise ValueError("Docs notice capture requires both rendered books")
    files["provenance.json"] = specification
    files["NOTICE.txt"] = (
        "PurRDF Book recipient notices\n============================\n\n"
        + declared["scope"]
        + "\n\n"
        + "The complete controlling texts and original copyrights accompany this notice.\n"
        + "The JavaScript, CSS and generated navigation/search data are shipped in readable source form.\n"
        + "Original mdBook source forms/templates accompany them in sources/mdbook/.\n"
        + "The rendered HTML contains first-party book text and the MPL-covered mdBook template;\n"
        + "the original template is retained and book sources are available at\n"
        + "https://github.com/Blackcat-Informatics/purrdf/tree/main/docs/book.\n"
        + "Original upstream mdBook sources: "
        + declared["mdbook_source"]
        + "\n\n"
        + "No third-party material is relicensed under the PurRDF SDK grant.\n"
        + "The console's own complete notices are at ../../playground/purrdf/licenses/NOTICE.txt.\n\n"
        + "\n".join(declared["limitations"])
        + "\n\n"
        + "\n".join(
            f"{item['name']}: {item['source_url']}"
            for item in declared["upstream_grants"]
        )
        + "\n"
    ).encode()
    files["inventory.json"] = (
        json.dumps(
            {
                "schema": 1,
                "profile": "docs",
                "mdbook_version": declared["mdbook_version"],
                "scope": declared["scope"],
                "assets": dict(sorted(assets.items())),
                "files": {name: digest(value) for name, value in sorted(files.items())},
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    ).encode()
    return files


def audit_pages(path: Path, recipient_root: str, expected: dict[str, bytes]) -> dict:
    receipt = audit_directory(path, recipient_root, expected)
    docs = audit_payload(path, "docs", docs_bundle(path), directory_files(path))
    return {
        **receipt,
        "docs_notice_files": docs["notice_files"],
        "docs_status": docs["status"],
    }


def docs_self_test() -> None:
    """Pure fixtures do not require Docs resources in source-only audit capsules."""
    with tempfile.TemporaryDirectory(prefix="purrdf-docs-notices-") as raw:
        root = Path(raw)
        source = root / "source"
        resources = source / "docs/book/recipient-notices"
        resources.mkdir(parents=True)
        (source / "Cargo.toml").write_text('[workspace.package]\nversion="3.0.0"\n')
        font = b"fixture font source"
        font_name = "fonts/font-" + digest(font)[:8] + ".woff2"
        style = b"url('{{ resource \"fonts/font.woff2\" }}');\n"
        style_name = "fonts/fonts-" + digest(style)[:8] + ".css"
        code = b"fixture unminified mdBook source"
        code_name = "book-" + digest(code)[:8] + ".js"
        grant = b"full controlling fixture text"
        files = {"MPL-2.0.txt": grant, "source.css.txt": style}
        for name, value in files.items():
            (resources / name).write_bytes(value)
        spec = {
            "schema": 1,
            "mdbook_version": "0.5.3",
            "mdbook_source": "https://example.org/mdbook",
            "scope": "fixture scope",
            "limitations": [],
            "upstream_grants": [],
            "files": {name: digest(value) for name, value in files.items()},
            "assets": [
                {
                    "name": font_name,
                    "source_sha256": digest(font),
                    "licenses": ["OFL-1.1"],
                    "source": "https://example.org/font",
                    "locales": ["", "zh-Hans"],
                },
                {
                    "name": code_name,
                    "source_sha256": digest(code),
                    "licenses": ["MPL-2.0"],
                    "source": "https://example.org/code",
                    "locales": ["", "zh-Hans"],
                },
                {
                    "name": style_name,
                    "source_sha256": digest(style),
                    "source_form": "source.css.txt",
                    "template": True,
                    "licenses": ["MPL-2.0"],
                    "source": "https://example.org/css",
                    "locales": ["", "zh-Hans"],
                },
            ],
        }
        (resources / "manifest.json").write_text(json.dumps(spec))
        book = root / "book"
        book.mkdir()
        for locale in ("", "zh-Hans/"):
            for name, value in {
                font_name: font,
                code_name: code,
                style_name: style.replace(
                    b'{{ resource "fonts/font.woff2" }}', ("../" + font_name).encode()
                ),
                "index.html": b"fixture first-party book",
            }.items():
                path = book / (locale + name)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(value)
        with patch.dict(globals(), ROOT=source):
            expected = docs_bundle(book)
            if expected != docs_bundle(book):
                raise ValueError("self-test: Docs notice capture is nondeterministic")
            destination = book / "recipient-notices/licenses"
            materialize(destination, expected, True)
            audit_payload(book, "docs", expected, directory_files(book))
            for member in (
                "MPL-2.0.txt",
                "source.css.txt",
                "provenance.json",
                "inventory.json",
            ):
                path = destination / member
                retained = path.read_bytes()
                path.unlink()
                try:
                    audit_payload(book, "docs", expected, directory_files(book))
                except ValueError:
                    pass
                else:
                    raise ValueError(
                        "self-test: incomplete Docs grant/source form accepted"
                    )
                path.write_bytes(retained)
            for member in (font_name, code_name, style_name, "zh-Hans/" + font_name):
                path = book / member
                retained = path.read_bytes()
                path.write_bytes(retained + b"altered")
                try:
                    docs_bundle(book)
                except ValueError:
                    pass
                else:
                    raise ValueError(
                        "self-test: changed pinned/generated Docs source accepted"
                    )
                path.write_bytes(retained)
            unknown = book / "unknown.js"
            unknown.write_bytes(b"undeclared foreign code")
            try:
                docs_bundle(book)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: unknown Docs frontend asset accepted")
            unknown.unlink()
            (book / "zh-Hans/index.html").unlink()
            try:
                docs_bundle(book)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: missing rendered locale accepted")
            (book / "zh-Hans/index.html").write_bytes(b"fixture first-party book")
            (resources / "MPL-2.0.txt").write_bytes(b"altered upstream grant")
            try:
                docs_bundle(book)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: changed pinned grant text accepted")
            (resources / "MPL-2.0.txt").write_bytes(grant)
            (resources / "linked-sources").symlink_to(
                resources, target_is_directory=True
            )
            try:
                docs_bundle(book)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: symlinked Docs source directory accepted")
            (resources / "linked-sources").unlink()
            original = resources.with_name("original-notices")
            resources.rename(original)
            resources.symlink_to(original, target_is_directory=True)
            try:
                docs_bundle(book)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: symlinked Docs evidence root accepted")
            resources.unlink()
            original.rename(resources)
    print(
        "Docs artifact self-test: full grants/source forms and deterministic resource expansions accepted; missing/corrupt texts/sources, altered static/generated/font assets, unknown assets and missing locale refused"
    )


def directory_self_test() -> None:
    """Exercise real directory audit boundaries without compiling a module."""
    with tempfile.TemporaryDirectory(prefix="purrdf-recipient-directory-") as raw:
        root = Path(raw)
        source = root / "source"
        source.mkdir()
        (source / "Cargo.toml").write_text('[workspace.package]\nversion="3.0.0"\n')
        npm = source / "crates/rdf-wasm/js"
        npm.mkdir(parents=True)
        compiler = b"rustc fixture\ncommit-hash: fixture\n"
        runtime = {
            "COPYRIGHT-library.html": b"runtime full report",
            "licenses/MIT.txt": b"runtime full terms",
        }
        runtime["inventory.json"] = json.dumps(
            {
                "compiler": compiler.decode(),
                "files": {name: digest(value) for name, value in runtime.items()},
            }
        ).encode()
        expected = {
            "LICENSE-MIT": b"complete base terms",
            "inventory.json": b'{"profile":"npm"}',
        }
        code = {
            "package.json": b'{"name":"@blackcatinformatics/purrdf","version":"3.0.0","license":"MIT OR Apache-2.0 OR MulanPSL-2.0"}',
            "index.mjs": b"fixture sdk",
            "pkg/purrdf_jspi.mjs": b"fixture jspi",
            "pkg/purrdf_wasm.js": b"fixture glue",
            "pkg/purrdf_wasm.d.ts": b"fixture types",
            "pkg/purrdf_wasm_bg.wasm": b"\0asm\x01\0\0\0fixture",
            "pkg/purrdf_wasm_bg.wasm.d.ts": b"fixture wasm types",
        }
        payload = {"purrdf/" + name: value for name, value in code.items()}
        for name, value in code.items():
            destination = npm / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(value)
        shell = source / "docs/playground"
        shell.mkdir(parents=True)
        for name in (
            "index.html",
            "app.mjs",
            "engine.worker.mjs",
            "sarif.mjs",
            "style.css",
            "sw.mjs",
            "manifest.webmanifest",
            "examples/gallery.mjs",
        ):
            destination = shell / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(b"shell fixture " + name.encode())
            payload[name] = destination.read_bytes()
        payload["purrdf/build-compiler.txt"] = compiler
        payload.update(
            {"purrdf/licenses/" + name: value for name, value in expected.items()}
        )
        payload.update(
            {
                "purrdf/licenses/runtime/" + name: value
                for name, value in runtime.items()
            }
        )
        backend = SimpleNamespace(runtime_bundle=lambda: runtime)

        def write_tree(path, values):
            path.mkdir()
            for name, value in values.items():
                destination = path / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(value)

        with (
            patch.dict(globals(), ROOT=source),
            patch.dict(globals(), python_backend=lambda: backend),
        ):
            valid = root / "valid"
            write_tree(valid, payload)
            first = audit_directory(valid, "purrdf", expected)
            if first != audit_directory(valid, "purrdf", expected):
                raise ValueError("self-test: directory evidence is nondeterministic")
            pages = root / "pages"
            write_tree(
                pages, {"playground/" + name: value for name, value in payload.items()}
            )
            audit_directory(pages, "playground/purrdf", expected)
            for index, (member, replacement) in enumerate(
                (
                    ("purrdf/licenses/LICENSE-MIT", None),
                    ("purrdf/licenses/LICENSE-MIT", b"corrupt"),
                    ("purrdf/licenses/runtime/COPYRIGHT-library.html", None),
                    ("purrdf/licenses/runtime/licenses/MIT.txt", b"corrupt"),
                    ("purrdf/package.json", None),
                    (
                        "purrdf/package.json",
                        b'{"name":"@blackcatinformatics/purrdf","version":"3.0.0"}',
                    ),
                    (
                        "purrdf/package.json",
                        b'{"name":"@blackcatinformatics/purrdf","version":"2.0.2"}',
                    ),
                    ("purrdf/build-compiler.txt", b"other compiler"),
                    ("purrdf/pkg/purrdf_wasm_bg.wasm", b"stale wasm"),
                    ("app.mjs", b"stale shell"),
                    (".stage/private.txt", b"private"),
                )
            ):
                values = dict(payload)
                if replacement is None:
                    del values[member]
                else:
                    values[member] = replacement
                path = root / f"refusal-{index}"
                write_tree(path, values)
                try:
                    audit_directory(path, "purrdf", expected)
                except ValueError:
                    pass
                else:
                    raise ValueError(f"self-test: directory accepted changed {member}")
            (valid / "linked-notice").symlink_to(valid / "purrdf/licenses/LICENSE-MIT")
            try:
                audit_directory(valid, "purrdf", expected)
            except ValueError:
                pass
            else:
                raise ValueError("self-test: directory accepted symlink evidence")
            (valid / "linked-notice").unlink()
            for relative in ("/purrdf", "../purrdf", "./purrdf", "purrdf/../purrdf"):
                try:
                    audit_directory(valid, relative, expected)
                except ValueError:
                    pass
                else:
                    raise ValueError("self-test: unsafe recipient root accepted")
        cargo = root / "cargo"
        cargo.write_bytes(b"\x7fELFfixture")
        home = root / "home"
        home.mkdir()
        config_dir = source / ".cargo"
        config_dir.mkdir()
        config = config_dir / "config.toml"
        config.write_text('[build]\nrustc-wrapper="kache"\n')
        environment = {name: "" for name in COMPILER_ENV}
        environment["CARGO_HOME"] = str(home)
        with (
            patch.dict(globals(), ROOT=source),
            patch.dict(os.environ, environment),
            patch.object(Path, "home", return_value=home),
            patch.object(shutil, "which", return_value=str(cargo)),
            patch.object(
                subprocess, "run", return_value=SimpleNamespace(stdout=compiler)
            ),
        ):
            if compiler_identity() != compiler:
                raise ValueError(
                    "self-test: explicit empty compiler wrapper was not honored"
                )
            for name in COMPILER_ENV:
                with patch.dict(os.environ, {name: "different-compiler"}):
                    try:
                        compiler_identity()
                    except ValueError:
                        pass
                    else:
                        raise ValueError(
                            "self-test: compiler environment redirection accepted"
                        )
            os.environ.pop("RUSTC_WRAPPER")
            try:
                compiler_identity()
            except ValueError:
                pass
            else:
                raise ValueError(
                    "self-test: configured wrapper without explicit reset accepted"
                )
            os.environ["RUSTC_WRAPPER"] = ""
            for value in (
                '[build]\nrustc="other"\n',
                'include=["other.toml"]\n',
                '[env]\nRUSTC="other"\n',
            ):
                config.write_text(value)
                try:
                    compiler_identity()
                except ValueError:
                    pass
                else:
                    raise ValueError("self-test: Cargo compiler redirection accepted")
            config.write_text("")
            cargo.write_bytes(b"#!/bin/sh\n")
            try:
                compiler_identity()
            except ValueError:
                pass
            else:
                raise ValueError("self-test: unverified Cargo launcher accepted")
            cargo.write_bytes(b"\x7fELFfixture")
            rustc = root / "rustc"
            rustc.write_bytes(b"#!/bin/sh\n")
            with patch.object(
                shutil, "which", side_effect=lambda name: str(root / name)
            ):
                try:
                    compiler_identity()
                except ValueError:
                    pass
                else:
                    raise ValueError("self-test: unverified rustc launcher accepted")
    print(
        "directory artifact self-test: console/Pages accepted deterministically; missing/corrupt base/runtime/metadata/compiler/module/shell, symlink/private/traversal and compiler redirection refused; explicit empty wrapper honored"
    )


def self_test() -> None:
    expected = {
        "LICENSE-MIT": b"permission notice\n",
        "inventory.json": b'{"profile":"fixture"}\n',
    }
    with tempfile.TemporaryDirectory(prefix="purrdf-license-audit-") as raw:
        root = Path(raw)
        for label, omitted, altered, success in (
            ("valid", None, False, True),
            ("omitted", "LICENSE-MIT", False, False),
            ("altered", None, True, False),
        ):
            path = root / f"{label}.tgz"
            with tarfile.open(path, "w:gz") as archive:
                for name, value in expected.items():
                    if name == omitted:
                        continue
                    value = b"different" if altered and name == "LICENSE-MIT" else value
                    item = tarfile.TarInfo(f"package/licenses/{name}")
                    item.size = len(value)
                    archive.addfile(item, io.BytesIO(value))
            try:
                audit(path, "fixture", expected)
                passed = True
            except ValueError:
                passed = False
            if passed != success:
                raise ValueError(
                    f"self-test {label}: acceptance {passed}, expected {success}"
                )
        wheel = root / "fixture.whl"
        with zipfile.ZipFile(wheel, "w") as archive:
            for name, value in expected.items():
                archive.writestr(f"fixture.dist-info/licenses/licenses/{name}", value)
        audit(wheel, "fixture", expected)
        incomplete = root / "incomplete-copy.whl"
        with zipfile.ZipFile(incomplete, "w") as archive:
            for name, value in expected.items():
                archive.writestr(f"fixture.dist-info/licenses/licenses/{name}", value)
            archive.writestr(
                "fixture/licenses/inventory.json", expected["inventory.json"]
            )
        try:
            audit(incomplete, "fixture", expected)
        except ValueError:
            pass
        else:
            raise ValueError(
                "self-test: a second incomplete recipient bundle was accepted"
            )
        # A linked artifact is a distinct boundary: correct source notices do
        # not excuse omitting the actual compiler runtime's recipient notices.
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"][
            "package"
        ]["version"]
        npm_expected = {**expected, "inventory.json": b'{"profile":"npm"}\n'}
        linked = root / "linked-runtime.tgz"
        payload = {
            f"package/licenses/{name}": value for name, value in npm_expected.items()
        }
        payload["package/package.json"] = json.dumps(
            {"name": "@blackcatinformatics/purrdf", "version": version}
        ).encode()
        payload["package/fixture.wasm"] = (
            b"\x00asm\x01\x00\x00\x00"  # legal-gate fixture, never a shipping module
        )
        with tarfile.open(linked, "w:gz") as archive:
            for name, value in payload.items():
                item = tarfile.TarInfo(name)
                item.size = len(value)
                archive.addfile(item, io.BytesIO(value))
        try:
            audit(linked, "npm", npm_expected)
        except ValueError as error:
            if "linked runtime notice" not in str(error):
                raise
        else:
            raise ValueError(
                "self-test: linked artifact without runtime notices accepted"
            )
        add_runtime(linked, "npm")
        audit(linked, "npm", npm_expected)
        qualified = linked.read_bytes()
        add_runtime(linked, "npm")
        if linked.read_bytes() != qualified:
            raise ValueError("self-test: runtime notice capture is not byte-idempotent")
        payload = archive_files(linked)
        del payload["package/licenses/runtime/COPYRIGHT-library.html"]
        with tarfile.open(linked, "w:gz") as archive:
            for name, value in payload.items():
                item = tarfile.TarInfo(name)
                item.size = len(value)
                archive.addfile(item, io.BytesIO(value))
        try:
            audit(linked, "npm", npm_expected)
        except ValueError as error:
            if "runtime/COPYRIGHT-library.html" not in str(error):
                raise
        else:
            raise ValueError(
                "self-test: linked artifact with deleted runtime report accepted"
            )
        for member in (
            "package/.stage/notes.txt",
            "package/ROADMAP-private.md",
            "package/vectors/xmlconf/xmltest/case.xml",
            "package/crates/text/lexicons/artifacts/dictionary.cbor",
            "../escape",
        ):
            path = root / "forbidden.tgz"
            with tarfile.open(path, "w:gz") as archive:
                item = tarfile.TarInfo(member)
                item.size = 1
                archive.addfile(item, io.BytesIO(b"x"))
            try:
                archive_files(path)
            except ValueError:
                pass
            else:
                raise ValueError(
                    "self-test: forbidden working or acquired material accepted"
                )
        destination = root / "licenses"
        materialize(destination, expected, True)
        if materialize(destination, expected, False):
            raise ValueError("self-test: valid generated bundle rejected")
        (destination / "LICENSE-MIT").unlink()
        if not materialize(destination, expected, False):
            raise ValueError("self-test: deleted generated bundle accepted")
    directory_self_test()
    docs_self_test()
    print(
        "license artifact self-test: valid tar/wheel accepted; missing/altered source and runtime notices, stale bundles, private/acquired payloads and traversal refused; runtime capture byte-idempotent"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--write", action="store_true")
    action.add_argument("--check", action="store_true")
    action.add_argument("--audit", nargs="+", type=Path, metavar="ARCHIVE")
    action.add_argument("--self-test", action="store_true")
    action.add_argument(
        "--compiler-record",
        type=Path,
        help="capture an identified compiler after refusing Cargo compiler redirection",
    )
    action.add_argument("--audit-directory", nargs="+", type=Path, metavar="DIRECTORY")
    action.add_argument(
        "--docs-dir",
        type=Path,
        help="capture pinned mdBook frontend grants and source forms after rendering both books",
    )
    action.add_argument(
        "--runtime-dir",
        type=Path,
        help="capture the current compiler's standard-library recipient notices",
    )
    action.add_argument(
        "--add-runtime",
        nargs="+",
        type=Path,
        help="add standard-library notices to npm/C archives before attestation",
    )
    parser.add_argument("--profile", help="explicit package profile for archive audits")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument(
        "--with-docs-assets",
        action="store_true",
        help="also audit the complete pinned mdBook frontend in a Pages tree",
    )
    parser.add_argument(
        "--recipient-root",
        help="relative npm recipient path inside an assembled playground or Pages tree",
    )
    parser.add_argument(
        "--require-cargo-set",
        action="store_true",
        help="require exactly all publishable Cargo archives",
    )
    args = parser.parse_args()
    if args.compiler_record:
        args.compiler_record.write_bytes(compiler_identity())
        return 0
    if args.docs_dir:
        materialize(
            args.docs_dir / "recipient-notices/licenses",
            docs_bundle(args.docs_dir),
            True,
        )
        print("Pinned mdBook recipient grants and source forms captured")
        return 0
    if args.with_docs_assets and not args.audit_directory:
        parser.error("Docs asset audit requires --audit-directory")
    if args.runtime_dir:
        materialize(args.runtime_dir, python_backend().runtime_bundle(), True)
        print("Rust standard-library recipient notices captured")
        return 0
    if args.add_runtime:
        if args.profile not in {"npm", "c"}:
            parser.error("archive runtime capture requires --profile npm or c")
        for path in args.add_runtime:
            add_runtime(path, args.profile)
        print("Rust standard-library notices added before artifact audit/attestation")
        return 0
    if args.self_test:
        self_test()
        return 0
    meta = metadata()
    inherited = inventory()
    targets = profiles(meta)
    if args.profile and args.profile not in targets:
        parser.error(f"unknown profile {args.profile!r}")
    selected = {args.profile: targets[args.profile]} if args.profile else targets
    expected = {
        name: bundle(name, details, meta, inherited)
        for name, details in selected.items()
    }
    if args.audit_directory:
        if args.profile != "npm" or not args.recipient_root or args.require_cargo_set:
            parser.error("directory audit requires --profile npm and --recipient-root")
        receipts = [
            (audit_pages if args.with_docs_assets else audit_directory)(
                path, args.recipient_root, expected["npm"]
            )
            for path in args.audit_directory
        ]
    elif args.audit:
        receipts = []
        for path in args.audit:
            if args.profile:
                name = args.profile
            elif path.suffix == ".crate":
                name = next(
                    (
                        key
                        for key in targets
                        if path.name.startswith(key + "-") and key.startswith("purrdf")
                    ),
                    None,
                )
                if not name:
                    raise ValueError(f"cannot identify crate archive {path}")
                # Prefix names overlap (purrdf and purrdf-core); use the longest.
                name = max(
                    (key for key in targets if path.name.startswith(key + "-")), key=len
                )
            else:
                parser.error("non-Cargo archives require --profile")
            receipts.append(audit(path, name, expected[name]))
        if args.require_cargo_set:
            required = {
                name
                for name in targets
                if name.startswith("purrdf") and name != "python-rdflib"
            }
            actual = [item["profile"] for item in receipts]
            if len(actual) != len(set(actual)) or set(actual) != required:
                raise ValueError(
                    "Cargo archive set is incomplete or duplicated: "
                    + ", ".join(sorted(required - set(actual)))
                )
    if args.audit or args.audit_directory:
        report = (
            json.dumps({"schema": 1, "artifacts": receipts}, indent=2, sort_keys=True)
            + "\n"
        )
        if args.receipt:
            args.receipt.parent.mkdir(parents=True, exist_ok=True)
            args.receipt.write_text(report)
        print(report, end="")
    else:
        problems = []
        for name, details in selected.items():
            problems.extend(
                materialize(details[0] / "licenses", expected[name], args.write)
            )
        if problems:
            raise ValueError(
                "\n".join(problems)
                + "\nRegenerate with python3 scripts/package-licenses.py --write"
            )
        print(
            f"license bundles {'generated' if args.write else 'verified'}: {len(selected)} profiles"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"package-licenses: {error}") from error
