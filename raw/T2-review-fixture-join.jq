def hexfield: ascii_downcase | if length == 0 then "\\0" else . end;
def field($v; $k): $v[$k] | hexfield;
($p[0].testGroups[] | select(
  if $op == "keyGen" then .tgId == 2
  elif $op == "sigGen" then .tgId == 3 or .tgId == 15
  else .tgId == 3 end)) as $g |
if ($g.parameterSet != "ML-DSA-65" or
    ($op != "keyGen" and ($g.signatureInterface != "external" or $g.preHash != "pure")))
then error("wrong group contract") else
($e[0].testGroups | map(select(.tgId == $g.tgId))) as $answers |
if ($answers | length) != 1 then error("answer group not unique") else
$g.tests[] as $case |
($answers[0].tests | map(select(.tcId == $case.tcId))) as $match |
if ($match | length) != 1 then error("answer case not unique") else
$match[0] as $answer |
([$g.tgId | tostring, $case.tcId | tostring] +
if $op == "keyGen" then
  [field($case; "seed"), field($answer; "pk"), field($answer; "sk")]
elif $op == "sigGen" then
  [field($case; "sk"), field($case; "message"), field($case; "context"),
   (if $g.deterministic then "00" * 32 else field($case; "rnd") end),
   field($answer; "signature")]
else
  [field($case; "pk"), field($case; "message"), field($case; "context"),
   field($case; "signature"), $answer.testPassed | tostring]
end) | join("\t")
end end end
