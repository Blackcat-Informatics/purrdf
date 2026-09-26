# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Unit tests for `check-wasm-jspi-frame.py` over hand-written `wasm-dis`-layout text.

Run with `python3 -m unittest scripts/test_check_wasm_jspi_frame.py`.
"""

from __future__ import annotations

import importlib.util
import io
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

_SPEC = importlib.util.spec_from_file_location(
    "check_wasm_jspi_frame", Path(__file__).with_name("check-wasm-jspi-frame.py")
)
assert _SPEC is not None and _SPEC.loader is not None
gate = importlib.util.module_from_spec(_SPEC)
# Registered before it runs: its dataclasses resolve their module through sys.modules.
sys.modules[_SPEC.name] = gate
_SPEC.loader.exec_module(gate)

# The fields every fixture shares: the import, the stack-pointer global, and the
# exported stack-pointer adjuster the gate resolves the global through.
_PRELUDE = """(module
 (type $0 (func (param i32 i32 i32) (result i32)))
 (import "./purrdf_jspi.mjs" "purrdf_jspi_suspend" (func $fimport$1 (param i32 i32 i32) (result i32)))
 (global $global$0 (mut i32) (i32.const 1048576))
 (global $global$1 (mut i32) (i32.const 0))
 (export "__wbindgen_add_to_stack_pointer" (func $sp_add))
 (func $sp_add (param $0 i32) (result i32)
  (global.set $global$0
   (i32.add
    (local.get $0)
    (global.get $global$0)
   )
  )
  (global.get $global$0)
 )
"""

# The Rust frame function as rustc emits it: a 16-byte frame, the import, a read of
# the frame, and the epilogue's restore.
_GOOD_FRAME = """ (func $suspend (param $0 i32) (param $1 i32) (result i32)
  (local $2 i32)
  (local $3 i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 16)
    )
   )
  )
  (i64.store offset=8
   (local.get $2)
   (i64.const 0)
  )
  (local.set $3
   (call $fimport$1
    (local.get $0)
    (local.get $1)
    (i32.add
     (local.get $2)
     (i32.const 8)
    )
   )
  )
  (local.set $3
   (i32.add
    (i32.load offset=8
     (local.get $2)
    )
    (local.get $3)
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $2)
    (i32.const 16)
   )
  )
  (local.get $3)
 )
"""

# The same frame after wasm-opt inlined it into its caller: the frame is set up in
# the call's operand block and torn down in the sibling operand block that follows the
# call — the restore comes first even though a call and a global.get follow later.
_INLINED_GOOD_FRAME = """ (func $caller (param $0 i32) (param $1 i32)
  (local $2 i32)
  (local $3 i32)
  (local $scratch i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 48)
    )
   )
  )
  (local.set $1
   (i32.add
    (call $fimport$1
     (local.get $0)
     (block (result i32)
      (global.set $global$0
       (local.tee $3
        (i32.sub
         (global.get $global$0)
         (i32.const 16)
        )
       )
      )
      (i64.store offset=8
       (local.get $3)
       (i64.const 0)
      )
      (local.get $1)
     )
     (i32.add
      (local.get $3)
      (i32.const 8)
     )
    )
    (block (result i32)
     (local.set $scratch
      (i32.load offset=8
       (local.get $3)
      )
     )
     (global.set $global$0
      (i32.add
       (local.get $3)
       (i32.const 16)
      )
     )
     (local.get $scratch)
    )
   )
  )
  (global.set $global$1
   (global.get $global$0)
  )
  (drop
   (call $sp_add
    (i32.const 0)
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $2)
    (i32.const 48)
   )
  )
 )
"""

# No frame: nothing restores the stack pointer before the function ends.
_MISSING_FRAME = """ (func $bare (param $0 i32) (param $1 i32) (result i32)
  (call $fimport$1
   (local.get $0)
   (local.get $1)
   (i32.const 0)
  )
 )
"""

# A call runs on the stale stack pointer before the restore.
_CALL_BEFORE_RESTORE = """ (func $early_call (param $0 i32) (param $1 i32) (result i32)
  (local $2 i32)
  (local $3 i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 16)
    )
   )
  )
  (local.set $3
   (call $fimport$1
    (local.get $0)
    (local.get $1)
    (local.get $2)
   )
  )
  (drop
   (call $sp_add
    (i32.const 0)
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $2)
    (i32.const 16)
   )
  )
  (local.get $3)
 )
"""

# The restore is computed from the stale global rather than the frame.
_GLOBAL_GET_BEFORE_RESTORE = """ (func $stale_read (param $0 i32) (param $1 i32) (result i32)
  (local $2 i32)
  (local $3 i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 16)
    )
   )
  )
  (local.set $3
   (call $fimport$1
    (local.get $0)
    (local.get $1)
    (local.get $2)
   )
  )
  (global.set $global$0
   (i32.add
    (global.get $global$0)
    (i32.const 16)
   )
  )
  (local.get $3)
 )
"""

# A conditional branch could skip the restore.
_BRANCH_BEFORE_RESTORE = """ (func $branchy (param $0 i32) (param $1 i32) (result i32)
  (local $2 i32)
  (local $3 i32)
  (global.set $global$0
   (local.tee $2
    (i32.sub
     (global.get $global$0)
     (i32.const 16)
    )
   )
  )
  (block $label$1
   (local.set $3
    (call $fimport$1
     (local.get $0)
     (local.get $1)
     (local.get $2)
    )
   )
   (br_if $label$1
    (local.get $3)
   )
   (global.set $global$0
    (i32.add
     (local.get $2)
     (i32.const 16)
    )
   )
  )
  (local.get $3)
 )
"""

# A function that merely exists: no caller of the import at all.
_UNRELATED = """ (func $unrelated (param $0 i32) (result i32)
  (i32.add
   (local.get $0)
   (i32.const 1)
  )
 )
"""


def _module(*functions: str, extra: str = "") -> str:
    return _PRELUDE + extra + "".join(functions) + ")\n"


class JspiFrameGateTest(unittest.TestCase):
    def test_good_frame_passes(self) -> None:
        report = gate.check_text(_module(_GOOD_FRAME))
        self.assertEqual(report.violations, ())
        self.assertEqual(report.call_sites, 1)
        self.assertEqual(report.import_function, "$fimport$1")
        self.assertEqual(report.stack_pointer, "$global$0")
        self.assertEqual(report.functions, ("$suspend",))

    def test_inlined_good_frame_passes(self) -> None:
        report = gate.check_text(_module(_UNRELATED, _INLINED_GOOD_FRAME))
        self.assertEqual(report.violations, ())
        self.assertEqual(report.functions, ("$caller",))

    def test_missing_frame_fails_at_the_end_of_the_function(self) -> None:
        report = gate.check_text(_module(_MISSING_FRAME))
        self.assertEqual(len(report.violations), 1)
        self.assertEqual(report.violations[0].function, "$bare")
        self.assertEqual(report.violations[0].offender, "the end of the function")

    def test_call_before_restore_fails(self) -> None:
        report = gate.check_text(_module(_CALL_BEFORE_RESTORE))
        self.assertEqual(len(report.violations), 1)
        self.assertEqual(report.violations[0].function, "$early_call")
        self.assertEqual(report.violations[0].offender, "call $sp_add")

    def test_global_get_before_restore_fails(self) -> None:
        report = gate.check_text(_module(_GLOBAL_GET_BEFORE_RESTORE))
        self.assertEqual(len(report.violations), 1)
        self.assertEqual(report.violations[0].offender, "global.get $global$0")

    def test_branch_before_restore_fails(self) -> None:
        report = gate.check_text(_module(_BRANCH_BEFORE_RESTORE))
        self.assertEqual(len(report.violations), 1)
        self.assertEqual(report.violations[0].offender, "br_if $label$1")

    def test_every_caller_is_checked(self) -> None:
        # One good caller does not vouch for a bad one.
        report = gate.check_text(_module(_GOOD_FRAME, _CALL_BEFORE_RESTORE))
        self.assertEqual(report.call_sites, 2)
        self.assertEqual([v.function for v in report.violations], ["$early_call"])

    def test_no_caller_at_all_fails(self) -> None:
        with self.assertRaisesRegex(gate.GateError, "compiled out"):
            gate.check_text(_module(_UNRELATED))

    def test_an_indirect_reference_fails(self) -> None:
        table = ' (table $0 1 1 funcref)\n (elem $0 (i32.const 1) $fimport$1)\n'
        with self.assertRaisesRegex(gate.GateError, "other than by a direct call"):
            gate.check_text(_module(_GOOD_FRAME, extra=table))

    def test_the_stack_pointer_is_the_global_the_export_sets(self) -> None:
        # Swap which global the adjuster sets: the good frame now restores the wrong one.
        swapped = _module(_GOOD_FRAME).replace(
            " (func $sp_add (param $0 i32) (result i32)\n  (global.set $global$0",
            " (func $sp_add (param $0 i32) (result i32)\n  (global.set $global$1",
        )
        report = gate.check_text(swapped)
        self.assertEqual(report.stack_pointer, "$global$1")
        self.assertEqual(len(report.violations), 1)
        self.assertEqual(report.violations[0].offender, "the end of the function")

    def test_main_reports_ok_and_fail(self) -> None:
        # The command-line entry prints one OK line or names the offending function.
        texts = {"good.wat": _module(_GOOD_FRAME), "bad.wat": _module(_MISSING_FRAME)}
        out, err = io.StringIO(), io.StringIO()
        with (
            mock.patch.object(gate, "disassemble", side_effect=lambda path: texts[path.name]),
            redirect_stdout(out),
            redirect_stderr(err),
        ):
            self.assertEqual(gate.main(["gate", "good.wat"]), 0)
            self.assertEqual(gate.main(["gate", "bad.wat"]), 1)
        self.assertIn("OK: JSPI frame gate: 1 call site(s)", out.getvalue())
        self.assertIn("function $bare", err.getvalue())

if __name__ == "__main__":
    unittest.main()
