// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::path::Path;
use purrdf_testkit::bench::counter::CountReport;

fn read(root: &Path, row: &str, family: &str) -> CountReport {
    let text = std::fs::read_to_string(root.join(format!("{row}-{family}.json"))).unwrap();
    let report = CountReport::from_json(&text).unwrap();
    assert_eq!(report.raw_samples().len(),10);
    report
}

fn main() {
    let root = std::env::args_os().nth(1).expect("count report directory");
    let root = Path::new(&root);
    let before = std::env::args_os().nth(2);
    if let Some(before) = before {
        let before = Path::new(&before);
        println!("row family before after relative-change interval");
        for row in ["integer_add","integer_mul","decimal_add","decimal_mul","decimal_div","parse_render","to_f64"] {
            for family in ["bounded","exact","value"] {
                let a = read(root,row,family);
                let b = read(before,row,family);
                let change = a.compare("before-reservation",&b,0x443).unwrap();
                println!("{row} {family} {:.4} {:.4} {:.5}% [{:.5}%,{:.5}%] {:?}",b.estimates().median,a.estimates().median,change.point*100.0,change.ci_low*100.0,change.ci_high*100.0,change.verdict);
            }
        }
        return;
    }
    println!("row bounded exact value exact-relative-to-bounded interval");
    for row in ["integer_add","integer_mul","decimal_add","decimal_mul","decimal_div","parse_render","to_f64"] {
        let base = read(root,row,"bounded");
        let exact = read(root,row,"exact");
        let value = read(root,row,"value");
        exact.check_compatible(&base).unwrap();
        let change = exact.compare("bounded",&base,0x443).unwrap();
        if row == "decimal_div" {
            assert!(value.check_compatible(&base).is_err(),"different precision laws must not compare");
        } else {
            value.check_compatible(&base).unwrap();
            value.compare("bounded",&base,0x443).unwrap();
        }
        println!("{row} {:.4} {:.4} {:.4} {:.5}% [{:.5}%,{:.5}%] {:?}",base.estimates().median,exact.estimates().median,value.estimates().median,change.point*100.0,change.ci_low*100.0,change.ci_high*100.0,change.verdict);
    }
}
