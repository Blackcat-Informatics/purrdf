# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Exercise atomic ref replication against real local Git repositories."""

from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import time
import unittest

spec = importlib.util.spec_from_file_location(
    "git_sync", Path(__file__).with_name("sync-git-refs.py")
)
git_sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(git_sync)


class GitSyncTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "source"
        self.remote = self.root / "destination.git"
        self.git(self.root, "init", "--initial-branch=main", str(self.source))
        self.git(self.root, "init", "--bare", "--initial-branch=main", str(self.remote))
        self.git(self.source, "config", "user.name", "Synthetic sync fixture")
        self.git(self.source, "config", "user.email", "fixture@example.org")
        self.commit("first")
        self.git(self.source, "tag", "light")
        self.git(self.source, "-c", "tag.gpgSign=false", "tag", "-a", "annotated",
                 "-m", "Synthetic annotated tag")
        self.git(self.source, "branch", "main-other")
        self.git(self.source, "branch", "feature")
        self.old_cwd = Path.cwd()
        os.chdir(self.source)
        self.addCleanup(os.chdir, self.old_cwd)

    def git(self, directory, *arguments):
        result = subprocess.run(
            ["git", "-C", str(directory), *arguments],
            capture_output=True, text=True, check=True,
        )
        return result.stdout.strip()

    def commit(self, content):
        (self.source / "example").write_text(content)
        self.git(self.source, "add", "example")
        # Preserve the host's normal signing and verification hooks. A fresh CI
        # runner creates unsigned synthetic fixture commits by its own default.
        self.git(self.source, "commit", "-m", content)
        return self.git(self.source, "rev-parse", "HEAD")

    def identities(self, directory):
        output = self.git(directory, "for-each-ref",
                          "--format=%(objectname)%09%(refname)")
        return {name: identity for identity, name in
                (line.split("\t") for line in output.splitlines())}

    def sync(self, **options):
        self.diagnostics = io.StringIO()
        with redirect_stderr(self.diagnostics), redirect_stdout(io.StringIO()):
            return git_sync.sync(str(self.remote), retry_delays=(0, 0), **options)

    def hook(self, name, body):
        path = self.remote / "hooks" / name
        path.write_text("#!/bin/sh\nset -eu\n" + body + "\n")
        path.chmod(0o755)

    def test_main_and_exact_tag_objects_only_with_idempotent_replay(self):
        source = self.identities(self.source)
        self.assertNotEqual(source["refs/tags/annotated"], source["refs/heads/main"])
        expected = {name: identity for name, identity in source.items()
                    if name == "refs/heads/main" or name.startswith("refs/tags/")}
        self.assertEqual(self.sync(), expected)
        self.assertEqual(self.identities(self.remote), expected)
        self.assertEqual(self.sync(), expected)
        self.assertEqual(self.identities(self.remote), expected)

    def test_fast_forward_preserves_destination_only_refs(self):
        self.sync()
        old_main = self.git(self.source, "rev-parse", "main")
        self.git(self.remote, "update-ref", "refs/heads/retained", old_main)
        self.git(self.remote, "update-ref", "refs/tags/retained", old_main)
        new_main = self.commit("second")
        self.git(self.source, "tag", "new")
        self.sync()
        actual = self.identities(self.remote)
        self.assertEqual(actual["refs/heads/main"], new_main)
        self.assertEqual(actual["refs/tags/new"], new_main)
        self.assertEqual(actual["refs/heads/retained"], old_main)
        self.assertEqual(actual["refs/tags/retained"], old_main)
        self.assertNotIn("refs/heads/main-other", actual)
        self.assertNotIn("refs/heads/feature", actual)

    def test_tag_collision_refuses_all_updates_atomically(self):
        self.sync()
        before = self.identities(self.remote)
        self.commit("new main and conflicting tag")
        self.git(self.source, "tag", "--force", "light")
        self.git(self.source, "tag", "fresh")
        with self.assertRaises(git_sync.SyncError):
            self.sync()
        self.assertEqual(self.identities(self.remote), before)

    def test_divergent_main_refuses_new_tag_atomically(self):
        self.sync()
        self.git(self.source, "checkout", "--orphan", "unrelated")
        different = self.commit("unrelated history")
        self.git(self.remote, "fetch", str(self.source), different)
        self.git(self.remote, "update-ref", "refs/heads/main", different)
        self.git(self.source, "checkout", "main")
        self.git(self.source, "tag", "fresh")
        before = self.identities(self.remote)
        with self.assertRaises(git_sync.SyncError):
            self.sync()
        self.assertEqual(self.identities(self.remote), before)

    def test_no_atomic_capability_fails_before_mutation(self):
        self.git(self.remote, "config", "receive.advertiseAtomic", "false")
        with self.assertRaises(git_sync.SyncError):
            self.sync()
        self.assertEqual(self.identities(self.remote), {})

    def test_rejected_push_retries_three_times_and_never_reports_success(self):
        self.hook("pre-receive", "echo attempt >> attempts\nexit 1")
        with self.assertRaisesRegex(git_sync.SyncError, "three attempts"):
            self.sync()
        self.assertEqual((self.remote / "attempts").read_text().splitlines(),
                         ["attempt"] * 3)
        self.assertEqual(self.identities(self.remote), {})
        self.assertEqual(self.diagnostics.getvalue().count("Sync attempt"), 3)

    def test_cli_rejected_push_exhaustion_exits_nonzero(self):
        self.hook("pre-receive", "echo attempt >> attempts\nexit 1")
        script = Path(__file__).with_name("sync-git-refs.py").resolve()
        result = subprocess.run(
            [os.sys.executable, str(script), str(self.remote)],
            text=True, capture_output=True, timeout=30,
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr.count("Sync attempt"), 3)
        self.assertIn("synchronization failed after three attempts", result.stderr)
        self.assertEqual((self.remote / "attempts").read_text().splitlines(),
                         ["attempt"] * 3)
        self.assertEqual(self.identities(self.remote), {})

    def test_successful_push_with_wrong_readback_still_fails(self):
        self.hook("post-receive", "git update-ref -d refs/tags/light")
        with self.assertRaisesRegex(git_sync.SyncError, "three attempts"):
            self.sync()
        self.assertIn("remote ref readback differed: refs/tags/light",
                      self.diagnostics.getvalue())
        self.assertNotIn("refs/tags/light", self.identities(self.remote))

    def test_timeout_kills_git_and_hook_children_and_fails(self):
        self.hook("pre-receive", "echo attempt >> attempts\nsleep 20\nexit 1")
        started = time.monotonic()
        with self.assertRaisesRegex(git_sync.SyncError, "three attempts"):
            self.sync(timeout=0.1)
        self.assertLess(time.monotonic() - started, 3)
        self.assertIn("timed out", self.diagnostics.getvalue())
        self.assertEqual(self.identities(self.remote), {})

    def test_main_is_required_even_when_similar_branch_exists(self):
        self.git(self.source, "checkout", "main-other")
        self.git(self.source, "branch", "-D", "main")
        with self.assertRaisesRegex(git_sync.SyncError, "no refs/heads/main"):
            self.sync()
        self.assertEqual(self.identities(self.remote), {})

    def test_shallow_source_refused_before_remote_changes(self):
        shallow = self.root / "shallow"
        self.git(self.root, "clone", "--depth=1", self.source.as_uri(), str(shallow))
        os.chdir(shallow)
        with self.assertRaisesRegex(git_sync.SyncError, "complete history"):
            self.sync()
        self.assertEqual(self.identities(self.remote), {})

    def test_server_echoed_secret_diagnostics_are_not_printed(self):
        diagnostic_marker = "synthetic-remote-diagnostic-marker"
        self.hook("pre-receive", f"echo '{diagnostic_marker}' >&2\nexit 1")
        with self.assertRaises(git_sync.SyncError):
            self.sync()
        self.assertNotIn(diagnostic_marker, self.diagnostics.getvalue())

    def test_cli_rejects_credential_url_without_exposing_it(self):
        script = Path(__file__).with_name("sync-git-refs.py").resolve()
        result = subprocess.run(
            [os.sys.executable, str(script),
             "https://synthetic-user:synthetic-password@example.org/repo.git"],
            text=True, capture_output=True,
        )
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("synthetic-password", result.stdout + result.stderr)
        self.assertNotIn("synthetic-user", result.stdout + result.stderr)
        self.assertEqual(self.identities(self.remote), {})


class WorkflowAuthenticationTests(unittest.TestCase):
    def setUp(self):
        workflow = Path(__file__).resolve().parents[1] / ".github/workflows/gitee.yml"
        lines = workflow.read_text().splitlines()
        start = next(index for index, line in enumerate(lines)
                     if line.strip() == "- name: Push and verify main and tags")
        run = next(index for index in range(start, len(lines))
                   if lines[index].strip() == "run: |")
        indentation = len(lines[run]) - len(lines[run].lstrip())
        end = next((index for index in range(run + 1, len(lines))
                    if lines[index].strip()
                    and len(lines[index]) - len(lines[index].lstrip()) <= indentation),
                   len(lines))
        self.step = textwrap.dedent("\n".join(lines[run + 1:end])) + "\n"
        opener = "cat > \"$askpass\" <<'ASKPASS'\n"
        self.askpass = self.step.split(opener, 1)[1].split("\nASKPASS", 1)[0] + "\n"
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.executable = self.root / "askpass"
        self.executable.write_text(self.askpass)
        self.executable.chmod(0o700)
        self.username = "synthetic-user '$\"$(uname)"
        self.password = "synthetic-password '$\"$(uname)"
        self.environment = dict(os.environ, GITEE_USERNAME=self.username,
                                GITEE_PASSWORD=self.password)

    def run_askpass(self, prompt):
        return subprocess.run([str(self.executable), prompt], env=self.environment,
                              text=True, capture_output=True)

    def test_actual_workflow_shell_and_askpass_are_valid(self):
        for script in (self.step, self.askpass):
            result = subprocess.run(["bash", "-n"], input=script,
                                    text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_askpass_preserves_literal_secret_characters(self):
        for prompt, expected in (
            ("Username for 'https://gitee.com': ", self.username),
            ("Password for 'https://synthetic@gitee.com': ", self.password),
        ):
            result = self.run_askpass(prompt)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, expected + "\n")
            self.assertEqual(result.stderr, "")

    def test_askpass_refuses_unexpected_hosts_without_secret_output(self):
        for prompt in (
            "Username for 'https://unexpected.example.org': ",
            "Password for 'https://synthetic@unexpected.example.org': ",
            "Password for 'https://synthetic@gitee.com.evil.example.org': ",
        ):
            result = self.run_askpass(prompt)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout + result.stderr, "")

    def test_workflow_refuses_missing_credentials_before_running_git(self):
        for name in ("GITEE_USERNAME", "GITEE_PASSWORD"):
            environment = dict(self.environment)
            environment[name] = ""
            result = subprocess.run(["bash", "-c", self.step], env=environment,
                                    cwd=self.root, text=True, capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn(f"Missing {name} secret", result.stderr)
            self.assertNotIn(self.username, result.stdout + result.stderr)
            self.assertNotIn(self.password, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
