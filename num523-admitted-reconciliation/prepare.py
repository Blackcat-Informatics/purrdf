from pathlib import Path
import difflib
import subprocess
root=Path('.')
stage=root/'.stage/sparql-eval-complete-bounded-workspace/num523-admitted-reconciliation'
patch=[]
def change(s,a,b):
    assert s.count(a)==1, (a[:90],s.count(a))
    return s.replace(a,b)
for name in ['crates/xsd/src/temporal.rs','crates/sparql-eval/src/expr.rs']:
    before=(root/name).read_text(); s=before
    if name.endswith('temporal.rs'):
        s=change(s,'XsdValue::DateTime(value) => {\n            if target == XsdDatatype::Time','XsdValue::DateTime(value) => {\n            if target == XsdDatatype::DateTimeStamp {\n                return Ok(value.tz.map(|_| source.clone()));\n            }\n            if target == XsdDatatype::Time')
        s=change(s,'if target == XsdDatatype::DateTime {\n                return Ok(Some(XsdValue::DateTime(DateTime {','if matches!(target, XsdDatatype::DateTime | XsdDatatype::DateTimeStamp) {\n                if target == XsdDatatype::DateTimeStamp && value.tz.is_none() {\n                    return Ok(None);\n                }\n                return Ok(Some(XsdValue::DateTime(DateTime {')
        s=change(s,'D::DateTime => read_datetime(lexical).map(XsdValue::DateTime),','D::DateTime | D::DateTimeStamp => {\n            read_datetime_as(datatype, lexical).map(XsdValue::DateTime)\n        }')
        s=change(s,'read_datetime(lexical).map_err(|error| error.into_owned(lexical))','read_datetime_as(XsdDatatype::DateTime, lexical).map_err(|error| error.into_owned(lexical))')
        s=change(s,'/// Parse the existing resident date surface.','/// Parse dateTimeStamp through the same reader, requiring an explicit timezone.\npub fn parse_datetime_stamp(lexical: &str) -> Result<DateTime, XsdError> {\n    read_datetime_as(XsdDatatype::DateTimeStamp, lexical)\n        .map_err(|error| error.into_owned(lexical))\n}\n/// Parse the existing resident date surface.')
        s=change(s,'fn read_datetime(s: &str) -> Result<DateTime, TemporalReadError> {\n    let dt = XsdDatatype::DateTime;','fn read_datetime_as(dt: XsdDatatype, s: &str) -> Result<DateTime, TemporalReadError> {')
        s=change(s,'let (time_no_tz, tz) = split_tz(dt, s, time_part)?;','let (time_no_tz, tz) = split_tz(dt, s, time_part)?;\n    if dt == XsdDatatype::DateTimeStamp && tz.is_none() {\n        return Err(TemporalReadError::invalid(dt, s, "dateTimeStamp requires a timezone"));\n    }')
        s=change(s,'XsdDatatype::DateTime => (MonthAction::Clamped, SecondAction::Free),','XsdDatatype::DateTime | XsdDatatype::DateTimeStamp => {\n            (MonthAction::Clamped, SecondAction::Free)\n        }')
    else:
        s=change(s,'calendar_cast_source(lexical, datatype_iri, source_datatype)','calendar_cast_source(datatype_iri, source_datatype)')
        s=change(s,'return Ok(Some(xsd_to_term(ctx, &value)?));','let text = purrdf_xsd::value::canonical_non_numeric(&value)\n                    .ok_or(EvalError::WorkspaceBoundOverflow)?;\n                return borrowed_xsd_text_to_term(ctx, &text, target.iri());')
        s=change(s,'match crate::parsed_value::ParsedValue::parse_coded(lexical, target, true, &ctx.growth)? {\n            Ok(value) => governed_xsd_to_term(ctx, &value),','match crate::parsed_value::ParsedValue::parse_coded(lexical, target, true, &ctx.growth)? {\n            Ok(value) if target.is_calendar() => {\n                let text = purrdf_xsd::value::canonical_non_numeric(&value)\n                    .ok_or(EvalError::WorkspaceBoundOverflow)?;\n                borrowed_xsd_text_to_term(ctx, &text, target.iri())\n            }\n            Ok(value) => governed_xsd_to_term(ctx, &value),')
        s=change(s,'/// [`CalendarSource`] for a literal spelled `lexical` with datatype IRI `datatype`','/// [`CalendarSource`] for a literal with datatype IRI `datatype`')
        s=change(s,'fn calendar_cast_source(\n    lexical: &str,\n    datatype: &str,\n    modelled: Option<XsdDatatype>,\n) -> CalendarSource {','fn calendar_cast_source(datatype: &str, modelled: Option<XsdDatatype>) -> CalendarSource {')
        s=change(s,'            Some("dateTimeStamp") if has_timezone(lexical) => {\n                CalendarSource::Value(XsdDatatype::DateTime)\n            }\n','')
        begin=s.index('/// Whether a calendar spelling ends in a timezone:');end=s.index('/// What [`cast_duration_or_binary`] decided.',begin)
        s=s[:begin]+s[end:]
        s=s.replace('and the rest (`xsd:dateTimeStamp`, the list types) cast to no','and the list types cast to no')
        s=change(s,'"dateTimeStamp" | "NMTOKENS" | "IDREFS" | "ENTITIES" => Some(CastRow::NonNumeric),','"NMTOKENS" | "IDREFS" | "ENTITIES" => Some(CastRow::NonNumeric),')
        main=subprocess.check_output(['git','show','866765b3:crates/sparql-eval/src/expr.rs'],text=True)
        start=main.index('            (\n                D::DateTime,\n                "2026-10-04T12:00:00Z",\n                D::DateTimeStamp,')
        end=main.index('            (D::Date, "2026-10-04Z", D::GMonthDay,',start)
        anchor='            (D::Date, "2026-10-04Z", D::GMonthDay,'
        s=change(s,anchor,main[start:end]+anchor)
    (stage/(Path(name).name+'.txt')).write_text(s)
    patch.extend(difflib.unified_diff(before.splitlines(True),s.splitlines(True),fromfile='a/'+name,tofile='b/'+name))
(stage/'admitted-reconciliation.patch').write_text(''.join(patch))
