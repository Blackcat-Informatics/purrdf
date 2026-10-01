# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""PEP 517 adapter that declares the licenses of each actual distribution.

Maturin builds the extension and source archive. This adapter retains that
backend and qualifies its metadata; no source grant or license text is changed.
CLI release builds call the same qualifier before their artifacts are audited
or attested. Source metadata declares the wheel's differing expression dynamic.
"""

from __future__ import annotations

import argparse
import base64
import csv
import email.parser
import email.policy
import fnmatch
import gzip
import hashlib
import io
import json
import re
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

import tomllib

PROJECT = Path(__file__).resolve().parent
OWNED = "MIT OR Apache-2.0 OR MulanPSL-2.0"


def runtime_bundle() -> dict[str, bytes]:
    """Retain the actual compiler distribution's standard-library notices."""
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    sysroot = Path(
        subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip()
    )
    directory = sysroot / "share/doc/rust"
    report = (directory / "COPYRIGHT-library.html").read_bytes()
    if b"Copyright notices for The Rust Standard Library" not in report:
        raise ValueError(
            "compiler distribution has no identifiable standard-library notice report"
        )
    files = {"COPYRIGHT-library.html": report}
    licenses = sorted((directory / "licenses").glob("*.txt"))
    if not licenses:
        raise ValueError("compiler distribution lacks its recipient license texts")
    files.update({f"licenses/{path.name}": path.read_bytes() for path in licenses})
    evidence = {
        "schema": 1,
        "compiler": compiler,
        "scope": "Standard-library copyright report and license texts supplied by the actual Rust compiler distribution. The complete report retains build and target alternatives; it does not claim every listed dependency occurs in this binary.",
        "source": "https://github.com/rust-lang/rust/tree/"
        + next(
            line.removeprefix("commit-hash: ")
            for line in compiler.splitlines()
            if line.startswith("commit-hash: ")
        ),
        "files": {
            name: hashlib.sha256(value).hexdigest()
            for name, value in sorted(files.items())
        },
    }
    files["inventory.json"] = (
        json.dumps(evidence, indent=2, sort_keys=True) + "\n"
    ).encode()
    return files


def combined(expressions: set[str]) -> str:
    if any(not expression or expression == "Unknown" for expression in expressions):
        raise ValueError("every distributed component must have a known license")
    # Cargo's table/emitter-derived grants repeat the same owned choice;
    # retain it once and each mandatory inherited component term separately.
    expanded = set()
    for expression in expressions:
        owned_prefix = f"({OWNED}) AND "
        components = expression.removeprefix(owned_prefix).split(" AND ")
        if expression.startswith(owned_prefix) and set(components) <= {
            "MIT",
            "Unicode-3.0",
        }:
            expanded.update((OWNED, *components))
        else:
            expanded.add(expression)
    ordered = [OWNED, *sorted(expanded - {OWNED})]
    return " AND ".join(
        f"({expression})"
        if " OR " in expression or " AND " in expression
        else expression
        for expression in ordered
    )


def wheel_target(filename: str) -> str:
    platform = filename.removesuffix(".whl").rsplit("-", 1)[-1]
    if "x86_64" in platform or "amd64" in platform:
        arch = "x86_64"
    elif "aarch64" in platform or "arm64" in platform:
        arch = "aarch64"
    elif "i686" in platform or platform == "win32":
        arch = "i686"
    else:
        raise ValueError(f"unsupported native wheel architecture: {platform}")
    if platform.startswith("win"):
        return f"{arch}-pc-windows-msvc"
    if platform.startswith("macosx"):
        return f"{arch}-apple-darwin"
    if "linux" in platform:
        return f"{arch}-unknown-linux-{'musl' if platform.startswith('musllinux') else 'gnu'}"
    raise ValueError(f"unsupported native wheel platform: {platform}")


def binary_expression(target: str | None, *, offline: bool = True) -> str:
    # Unlike all-workspace metadata, Cargo's selected-root tree excludes weak
    # optional edges that remain in the lockfile but are not compiled.
    result = subprocess.run(
        [
            "cargo",
            "tree",
            "--locked",
            *(["--offline"] if offline else []),
            "--manifest-path",
            str(PROJECT / "Cargo.toml"),
            "-p",
            "purrdf-python",
            *(["--target", target] if target else []),
            "--edges",
            "normal,no-proc-macro",
            "--prefix",
            "none",
            "--format",
            "{p}|{l}",
        ],
        cwd=PROJECT,
        text=True,
        capture_output=True,
        check=True,
    )
    expressions = {OWNED, "Apache-2.0 OR MIT", "Unicode-3.0"}
    for row in result.stdout.splitlines():
        _, separator, license_expression = row.partition("|")
        if not separator:
            raise ValueError("unexpected Cargo license-tree output")
        expressions.add(license_expression.removesuffix(" (*)"))
    evidence = json.loads((PROJECT / "licenses/inventory.json").read_text())
    for component in evidence["components"]:
        if component["id"] in {"unicode", "libyaml-emitter"}:
            expressions.add(component["license"])
    return combined(expressions)


def source_expression(contents: dict[str, bytes]) -> str:
    expressions = {OWNED}
    relative = {
        name.split("/", 1)[1]: value for name, value in contents.items() if "/" in name
    }
    evidence = json.loads(relative["bindings/python/licenses/inventory.json"])
    if any(
        re.search(rb"SPDX-License-Identifier:\s*CC-BY-4\.0", value[:2048])
        for value in relative.values()
    ):
        expressions.add("CC-BY-4.0")
    for component in evidence["components"]:
        if not component.get("paths"):
            continue  # dependency notices alone do not redistribute its source
        if any(
            fnmatch.fnmatchcase(name, pattern)
            for name in relative
            for pattern in component["paths"]
        ):
            expressions.add(component["license"])
    # Every first-party crate whose sources are present retains its declared
    # derived-data obligation, independently of the conformance-data inventory.
    for name, value in relative.items():
        if name.endswith("/Cargo.toml"):
            declared = tomllib.loads(value.decode()).get("package", {}).get("license")
            if isinstance(declared, str):
                expressions.add(declared)
    return combined(expressions)


def metadata_bytes(raw: bytes, expression: str, *, source: bool) -> bytes:
    message = email.parser.BytesParser(policy=email.policy.default).parsebytes(raw)
    if message.get("License") is not None:
        del message["License"]
    if message.get("License-Expression") is not None:
        del message["License-Expression"]
    message["License-Expression"] = expression
    dynamic = message.get_all("Dynamic", [])
    if dynamic:
        del message["Dynamic"]
    licensing_fields = ("License-Expression", "License-File")
    for field in dynamic:
        if field.casefold() not in {name.casefold() for name in licensing_fields}:
            message["Dynamic"] = field
    if source:
        # Wheels add the actual compiler's recipient notices. Both the archive
        # expression and this multiple-use file list may therefore grow/change.
        for field in licensing_fields:
            message["Dynamic"] = field
    return serialize_metadata(message)


def serialize_metadata(message) -> bytes:
    # Header order has no semantic meaning; stable ordering also makes a second
    # qualification byte-identical when new License-File fields were added.
    fields = sorted(
        message.items(),
        key=lambda item: (item[0] != "Metadata-Version", item[0].casefold()),
    )
    for name in set(message.keys()):
        del message[name]
    for name, value in fields:
        message[name] = str(value)
    return message.as_bytes(
        policy=email.policy.default.clone(linesep="\n", max_line_length=0)
    )


def configured() -> None:
    project = tomllib.loads((PROJECT / "pyproject.toml").read_text())
    if project["build-system"].get(
        "build-backend"
    ) != "purrdf_build_backend" or project["build-system"].get("backend-path") != ["."]:
        raise ValueError(
            "native Python distribution must use its local license-aware PEP517 backend"
        )
    if (
        "license" not in project["project"].get("dynamic", [])
        or "license" in project["project"]
    ):
        raise ValueError(
            "native Python project must declare its per-artifact license dynamic"
        )


def qualify(path: Path) -> None:
    configured()
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            infos = archive.infolist()
            contents = {item.filename: archive.read(item) for item in infos}
        candidates = [name for name in contents if name.endswith(".dist-info/METADATA")]
        if len(candidates) != 1:
            raise ValueError("wheel must contain one core metadata file")
        name = candidates[0]
        roots = [
            name.removesuffix("inventory.json")
            for name, value in contents.items()
            if name.endswith("/licenses/inventory.json")
            and json.loads(value).get("profile") == "python"
        ]
        if not roots:
            raise ValueError("native wheel lacks its recipient notice bundle")
        runtime = runtime_bundle()
        for prefix in roots:
            for relative, value in runtime.items():
                name = f"{prefix}runtime/{relative}"
                if name not in contents:
                    info = zipfile.ZipInfo(name)
                    info.compress_type = zipfile.ZIP_DEFLATED
                    info.external_attr = 0o100644 << 16
                    infos.append(info)
                contents[name] = value
        name = candidates[0]
        contents[name] = metadata_bytes(
            contents[name], binary_expression(wheel_target(path.name)), source=False
        )
        message = email.parser.BytesParser(policy=email.policy.default).parsebytes(
            contents[name]
        )
        declared_files = set(message.get_all("License-File", []))
        license_root = name.removesuffix("METADATA") + "licenses/"
        for prefix in sorted(roots):
            for relative in sorted(runtime):
                member = f"{prefix}runtime/{relative}"
                if not member.startswith(license_root):
                    raise ValueError(
                        "runtime notices must occupy the wheel's PEP639 license directory"
                    )
                license_file = member.removeprefix(license_root)
                if license_file not in declared_files:
                    message["License-File"] = license_file
                    declared_files.add(license_file)
        contents[name] = serialize_metadata(message)
        records = [name for name in contents if name.endswith(".dist-info/RECORD")]
        if len(records) != 1:
            raise ValueError("wheel must contain one RECORD")
        record = records[0]
        output = io.StringIO(newline="")
        writer = csv.writer(output, lineterminator="\n")
        for name, value in sorted(contents.items()):
            if name != record:
                digest = (
                    base64.urlsafe_b64encode(hashlib.sha256(value).digest())
                    .rstrip(b"=")
                    .decode()
                )
                writer.writerow((name, f"sha256={digest}", len(value)))
        writer.writerow((record, "", ""))
        contents[record] = output.getvalue().encode()
        with tempfile.NamedTemporaryFile(
            dir=path.parent, suffix=".whl", delete=False
        ) as sink:
            temporary = Path(sink.name)
        try:
            with zipfile.ZipFile(
                temporary, "w", compression=zipfile.ZIP_DEFLATED
            ) as archive:
                for item in infos:
                    archive.writestr(item, contents[item.filename])
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)
    else:
        with tarfile.open(path) as archive:
            infos = archive.getmembers()
            contents = {
                item.name: archive.extractfile(item).read()
                for item in infos
                if item.isfile()
            }
        candidates = [
            name
            for name in contents
            if name.count("/") == 1 and name.endswith("/PKG-INFO")
        ]
        if len(candidates) != 1:
            raise ValueError("sdist must contain one core metadata file")
        name = candidates[0]
        contents[name] = metadata_bytes(
            contents[name], source_expression(contents), source=True
        )
        with tempfile.NamedTemporaryFile(
            dir=path.parent, suffix=".tar.gz", delete=False
        ) as sink:
            temporary = Path(sink.name)
        try:
            with (
                temporary.open("wb") as sink,
                gzip.GzipFile(
                    fileobj=sink, filename="", mode="wb", mtime=0
                ) as compressed,
                tarfile.open(fileobj=compressed, mode="w") as archive,
            ):
                for item in infos:
                    if item.isfile():
                        value = contents[item.name]
                        item.size = len(value)
                        archive.addfile(item, io.BytesIO(value))
                    else:
                        archive.addfile(item)
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)


def expected(path: Path, contents: dict[str, bytes]) -> str:
    return (
        binary_expression(wheel_target(path.name))
        if path.name.endswith(".whl")
        else source_expression(contents)
    )


def backend():
    import maturin

    return maturin


def get_requires_for_build_wheel(config_settings=None):
    return backend().get_requires_for_build_wheel(config_settings)


def get_requires_for_build_editable(config_settings=None):
    return backend().get_requires_for_build_editable(config_settings)


def get_requires_for_build_sdist(config_settings=None):
    return backend().get_requires_for_build_sdist(config_settings)


def prepare_metadata_for_build_wheel(metadata_directory, config_settings=None):
    return _prepare_metadata(metadata_directory, config_settings, editable=False)


def prepare_metadata_for_build_editable(metadata_directory, config_settings=None):
    return _prepare_metadata(metadata_directory, config_settings, editable=True)


def _prepare_metadata(metadata_directory, config_settings, *, editable):
    delegate = backend()
    hook = (
        delegate.prepare_metadata_for_build_editable
        if editable
        else delegate.prepare_metadata_for_build_wheel
    )
    name = hook(metadata_directory, config_settings)
    arguments = delegate.get_maturin_pep517_args(config_settings)
    target = None
    for index, argument in enumerate(arguments):
        if argument == "--target":
            target = arguments[index + 1]
        elif argument.startswith("--target="):
            target = argument.removeprefix("--target=")
    path = Path(metadata_directory) / name / "METADATA"
    message = email.parser.BytesParser(policy=email.policy.default).parsebytes(
        metadata_bytes(
            path.read_bytes(), binary_expression(target, offline=False), source=False
        )
    )
    declared = set(message.get_all("License-File", []))
    for relative, value in sorted(runtime_bundle().items()):
        license_file = f"licenses/runtime/{relative}"
        if license_file not in declared:
            message["License-File"] = license_file
        destination = path.parent / "licenses" / license_file
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(value)
    path.write_bytes(serialize_metadata(message))
    return name


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    return _build_wheel(
        wheel_directory, config_settings, metadata_directory, editable=False
    )


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    return _build_wheel(
        wheel_directory, config_settings, metadata_directory, editable=True
    )


def _build_wheel(wheel_directory, config_settings, metadata_directory, *, editable):
    """Capture one frontend's wheel before qualifying or exposing its bytes."""
    delegate = backend()
    arguments = list(delegate.get_maturin_pep517_args(config_settings))
    if any(
        argument in {"--out", "-o"} or argument.startswith(("--out=", "-o"))
        for argument in arguments
    ):
        raise ValueError("the PEP frontend owns the wheel output directory; omit --out")
    output = Path(wheel_directory).resolve()
    output.mkdir(parents=True, exist_ok=True)
    prepared = Path(metadata_directory).resolve() if metadata_directory else None
    metadata = {}
    if prepared is not None:
        if not prepared.is_dir() or not prepared.name.endswith(".dist-info"):
            raise ValueError(
                "prepared metadata must be its existing dist-info directory"
            )
        metadata = {
            path.relative_to(prepared).as_posix(): path.read_bytes()
            for path in prepared.rglob("*")
            if path.is_file()
        }
        if "METADATA" not in metadata:
            raise ValueError("prepared dist-info directory lacks METADATA")
    with tempfile.TemporaryDirectory(prefix=".purrdf-wheel-", dir=output) as directory:
        private = Path(directory)
        built = private / "built"
        recipient = private / "recipient"
        built.mkdir()
        recipient.mkdir()
        settings = dict(config_settings or {})
        settings["maturin.build-args"] = [*arguments, "--out", str(built)]
        hook = delegate.build_editable if editable else delegate.build_wheel
        name = hook(str(recipient), settings, metadata_directory)
        if Path(name).name != name or not name.endswith(".whl"):
            raise ValueError("Maturin must return one wheel basename")
        candidate = recipient / name
        qualify(candidate)
        if prepared is not None:
            with zipfile.ZipFile(candidate) as archive:
                for relative, expected in metadata.items():
                    member = f"{prepared.name}/{relative}"
                    if archive.read(member) != expected:
                        raise ValueError(
                            f"prepared editable/wheel metadata changed: {relative}"
                        )
        candidate.replace(output / name)
    return name


def build_sdist(sdist_directory, config_settings=None):
    name = backend().build_sdist(sdist_directory, config_settings)
    qualify(Path(sdist_directory) / name)
    return name


def self_test() -> None:
    """Exercise metadata variability and the wheel's independent integrity law."""
    raw = b"Metadata-Version: 2.4\nName: fixture\nVersion: 1\nLicense: obsolete\nDynamic: Requires-Dist\n\nREADME body\n"
    expression = combined({OWNED, "Unicode-3.0", "MIT"})
    source = metadata_bytes(raw, expression, source=True)
    message = email.parser.BytesParser(policy=email.policy.default).parsebytes(source)
    assert message["License"] is None
    assert message["License-Expression"] == expression
    assert message.get_all("Dynamic") == [
        "Requires-Dist",
        "License-Expression",
        "License-File",
    ]
    assert source == metadata_bytes(source, expression, source=True)
    assert message.get_payload() == "README body\n"
    wheel_metadata = metadata_bytes(source, expression, source=False)
    assert wheel_metadata == metadata_bytes(wheel_metadata, expression, source=False)
    wheel_message = email.parser.BytesParser(policy=email.policy.default).parsebytes(
        wheel_metadata
    )
    assert wheel_message.get_all("Dynamic") == ["Requires-Dist"]
    assert (
        wheel_target("fixture-1-cp313-cp313-musllinux_1_2_aarch64.whl")
        == "aarch64-unknown-linux-musl"
    )
    assert (
        wheel_target("fixture-1-cp313-cp313-win_amd64.whl") == "x86_64-pc-windows-msvc"
    )
    with tempfile.TemporaryDirectory(prefix="purrdf-license-backend-") as directory:
        path = Path(directory) / "fixture-1-cp313-cp313-manylinux_2_28_x86_64.whl"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("fixture.py", b"value = 1\n")
            archive.writestr(
                "fixture-1.dist-info/licenses/licenses/inventory.json",
                b'{"profile":"python"}\n',
            )
            archive.writestr("fixture-1.dist-info/METADATA", raw)
            archive.writestr("fixture-1.dist-info/RECORD", b"stale\n")
        qualify(path)
        first = path.read_bytes()
        from types import SimpleNamespace
        from unittest.mock import Mock, patch

        settings = {"maturin.build-args": "--target x86_64-unknown-linux-gnu"}
        prepared = Path(directory) / "fixture-1.dist-info"
        prepared.mkdir()
        (prepared / "METADATA").write_bytes(raw)
        calls = []

        def editable_fixture(wheel_directory, config_settings, metadata_directory):
            calls.append((wheel_directory, config_settings, metadata_directory))
            destination = Path(wheel_directory) / path.name
            destination.write_bytes(first)
            assert Path(config_settings["maturin.build-args"][-1]).is_dir()
            return path.name

        delegate = SimpleNamespace(
            get_requires_for_build_editable=Mock(return_value=["fixture>=1"]),
            prepare_metadata_for_build_editable=Mock(return_value=prepared.name),
            get_maturin_pep517_args=Mock(
                return_value=["--target", "x86_64-unknown-linux-gnu"]
            ),
            build_editable=editable_fixture,
        )
        with patch(__name__ + ".backend", return_value=delegate):
            assert get_requires_for_build_editable(settings) == ["fixture>=1"]
            assert (
                prepare_metadata_for_build_editable(directory, settings)
                == prepared.name
            )
            delegate.get_requires_for_build_editable.assert_called_once_with(settings)
            delegate.prepare_metadata_for_build_editable.assert_called_once_with(
                directory, settings
            )
            assert build_editable(directory, settings, str(prepared)) == path.name
            assert len(calls) == 1
            recipient, captured_settings, forwarded_metadata = calls[0]
            assert Path(recipient).parent.parent == Path(directory)
            assert captured_settings["maturin.build-args"][:2] == [
                "--target",
                "x86_64-unknown-linux-gnu",
            ]
            assert captured_settings["maturin.build-args"][-2] == "--out"
            assert forwarded_metadata == str(prepared)
            assert not Path(recipient).exists()
            with patch.object(
                delegate, "get_maturin_pep517_args", return_value=["--out=shared"]
            ):
                try:
                    build_editable(directory, settings)
                except ValueError:
                    pass
                else:
                    raise AssertionError("caller-selected shared output was accepted")
        assert path.read_bytes() == first
        prepared_metadata = email.parser.BytesParser(
            policy=email.policy.default
        ).parsebytes((prepared / "METADATA").read_bytes())
        assert prepared_metadata["License-Expression"] == binary_expression(
            "x86_64-unknown-linux-gnu"
        )
        assert "License-Expression" not in prepared_metadata.get_all("Dynamic", [])
        assert "License-File" not in prepared_metadata.get_all("Dynamic", [])
        for license_file in prepared_metadata.get_all("License-File", []):
            assert (prepared / "licenses" / license_file).is_file()
        qualify(path)
        assert path.read_bytes() == first
        with zipfile.ZipFile(path) as archive:
            record = "fixture-1.dist-info/RECORD"
            entries = list(csv.reader(io.StringIO(archive.read(record).decode())))
            assert {row[0] for row in entries} == set(archive.namelist())
            for name, checksum, size in entries:
                if name == record:
                    assert (checksum, size) == ("", "")
                else:
                    value = archive.read(name)
                    digest = (
                        base64.urlsafe_b64encode(hashlib.sha256(value).digest())
                        .rstrip(b"=")
                        .decode()
                    )
                    assert checksum == f"sha256={digest}" and size == str(len(value))
    print(
        "Python license backend self-test: dynamic source metadata, PEP660 delegation, target selection and wheel RECORD integrity passed"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archives", nargs="*", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    if not args.archives and not args.self_test:
        parser.error("supply distribution archives or --self-test")
    for path in args.archives:
        qualify(path)
        print(f"qualified distribution license metadata: {path.name}")


if __name__ == "__main__":
    main()
