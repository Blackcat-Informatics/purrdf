# Applied XML precision proof correction

Parent review found that the raw source predicate treated nested member
MaxCount(0) as an emitted XML exclusion and ignored effective severity. The
exclusion proof now lives beside the actual JSON Schema emission rules. It
uses their one constraint_severity home/default conformance set, distinguishes
property counts from counts dropped in member value schemas, and cannot use
computed/default/deactivated property shapes. Conditional Xone/negation
projection supplies no raw-source proof.

Actual projected-validator fixtures cover Node/And/Or member MaxCount(0),
shape/member custom severity, per-constraint reifier severity and computed
default: balanced and malformed XML remain admitted and coverage approximate.
The enabling reifier control and real datatype/member-datatype/property-max0
controls exclude XML and retain exact coverage. Rustfmt/diff checks PASS;
runtime remains NOT MET until the admitted qualification lane runs.
