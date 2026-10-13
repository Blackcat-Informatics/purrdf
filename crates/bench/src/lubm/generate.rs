// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{Draw, Spec};
use purrdf_iri::vocab::{owl, rdf};
use std::io::{self, Write};

struct Emitter<'a, W> {
    output: &'a mut W,
    schema: String,
    scratch: String,
    external: std::collections::BTreeSet<String>,
}
impl<W: Write> Emitter<'_, W> {
    fn edge(&mut self, subject: &str, predicate: &str, object: &str) -> io::Result<()> {
        self.scratch.clear();
        for iri in [subject, predicate, object] {
            purrdf_lex::term_syntax::write_iri(iri, &mut self.scratch);
            self.scratch.push(' ');
        }
        self.scratch.push_str(".\n");
        self.output.write_all(self.scratch.as_bytes())
    }
    fn relation(&mut self, subject: &str, property: &str, object: &str) -> io::Result<()> {
        self.edge(subject, &format!("{}{property}", self.schema), object)
    }
    fn class(&mut self, subject: &str, class: &str) -> io::Result<()> {
        self.edge(subject, rdf::TYPE, &format!("{}{class}", self.schema))
    }
    fn label(&mut self, subject: &str, property: &str, value: &str) -> io::Result<()> {
        self.scratch.clear();
        purrdf_lex::term_syntax::write_iri(subject, &mut self.scratch);
        self.scratch.push(' ');
        purrdf_lex::term_syntax::write_iri(
            &format!("{}{property}", self.schema),
            &mut self.scratch,
        );
        self.scratch.push(' ');
        purrdf_lex::term_syntax::write_literal(
            value,
            purrdf_xsd::datatype::XSD_STRING,
            None,
            None,
            &mut self.scratch,
        );
        self.scratch.push_str(" .\n");
        self.output.write_all(self.scratch.as_bytes())
    }
    fn person(&mut self, iri: &str, class: &str, department: &str) -> io::Result<()> {
        self.class(iri, class)?;
        let local = iri.rsplit('/').next().expect("person has local name");
        self.label(iri, "name", local)?;
        self.label(iri, "emailAddress", &format!("{local}@example.org"))?;
        self.label(iri, "telephone", "000-000-0000")?;
        self.relation(
            iri,
            if class.ends_with("Student") {
                "memberOf"
            } else {
                "worksFor"
            },
            department,
        )
    }
    fn degree(&mut self, person: &str, property: &str, draw: &mut Draw) -> io::Result<()> {
        let university = format!("http://www.ExternalUniversity{}.edu", draw.between(0, 999));
        if self.external.insert(university.clone()) {
            self.class(&university, "University")?;
        }
        self.relation(person, property, &university)
    }
}

pub(super) fn university<W: Write>(
    spec: &Spec,
    index: u64,
    mut file: impl FnMut(usize, &mut dyn FnMut(&mut W) -> io::Result<()>) -> io::Result<()>,
) -> io::Result<()> {
    let mut draw = Draw::new(spec.seed, index);
    let departments = draw.between(15, 25);
    for department in 0..departments {
        let mut emit =
            |output: &mut W| department_document(spec, index, department, &mut draw, output);
        file(department, &mut emit)?;
    }
    Ok(())
}

fn department_document<W: Write>(
    spec: &Spec,
    index: u64,
    number: usize,
    draw: &mut Draw,
    output: &mut W,
) -> io::Result<()> {
    let university = format!("http://www.University{index}.edu");
    let department = format!("http://www.Department{number}.University{index}.edu");
    let entity = |class: &str, n: usize| format!("{department}/{class}{n}");
    let mut out = Emitter {
        output,
        schema: format!("{}#", spec.ontology),
        scratch: String::with_capacity(512),
        external: std::collections::BTreeSet::new(),
    };
    let document = format!("{}University{index}_{number}.nt", spec.document_base);
    out.edge(&document, rdf::TYPE, owl::ONTOLOGY)?;
    out.edge(&document, owl::IMPORTS, &spec.ontology)?;
    out.class(&university, "University")?;
    out.label(&university, "name", &format!("University{index}"))?;
    out.class(&department, "Department")?;
    out.label(&department, "name", &format!("Department{number}"))?;
    out.relation(&department, "subOrganizationOf", &university)?;
    let categories = [
        ("FullProfessor", draw.between(7, 10), 15, 20),
        ("AssociateProfessor", draw.between(10, 14), 10, 18),
        ("AssistantProfessor", draw.between(8, 11), 5, 10),
        ("Lecturer", draw.between(5, 7), 0, 5),
    ];
    let faculty_count: usize = categories.iter().map(|category| category.1).sum();
    let mut professors = Vec::new();
    let mut undergraduate_courses = Vec::new();
    let mut graduate_courses = Vec::new();
    let mut professor_publications = Vec::new();
    let mut publication_count = 0;
    let head = draw.between(0, categories[0].1 - 1);
    for (class, count, low, high) in categories {
        for n in 0..count {
            let person = entity(class, n);
            out.person(&person, class, &department)?;
            if class != "Lecturer" {
                professors.push(person.clone());
            }
            if class == "FullProfessor" && n == head {
                out.relation(&person, "headOf", &department)?;
            }
            for (course_class, courses) in [
                ("Course", &mut undergraduate_courses),
                ("GraduateCourse", &mut graduate_courses),
            ] {
                for _ in 0..draw.between(1, 2) {
                    let course = entity(course_class, courses.len());
                    out.class(&course, course_class)?;
                    out.label(&course, "name", &format!("{course_class}{}", courses.len()))?;
                    out.relation(&person, "teacherOf", &course)?;
                    courses.push(course);
                }
            }
            for _ in 0..draw.between(low, high) {
                let publication = entity("Publication", publication_count);
                publication_count += 1;
                out.class(&publication, "Publication")?;
                out.label(
                    &publication,
                    "name",
                    &format!("Publication{}", publication_count - 1),
                )?;
                out.relation(&publication, "publicationAuthor", &person)?;
                if class != "Lecturer" {
                    professor_publications.push(publication);
                }
            }
            for property in [
                "undergraduateDegreeFrom",
                "mastersDegreeFrom",
                "doctoralDegreeFrom",
            ] {
                out.degree(&person, property, draw)?;
            }
            out.label(
                &person,
                "researchInterest",
                &format!("Research{}", draw.between(0, 99)),
            )?;
        }
    }
    for n in 0..draw.between(10, 20) {
        let group = entity("ResearchGroup", n);
        out.class(&group, "ResearchGroup")?;
        out.relation(&group, "subOrganizationOf", &department)?;
    }
    let undergraduates = draw.between(8 * faculty_count, 14 * faculty_count);
    let graduates = draw.between(3 * faculty_count, 4 * faculty_count);
    let advised: std::collections::BTreeSet<_> = draw
        .sample(undergraduates, undergraduates / 5)
        .into_iter()
        .collect();
    for n in 0..undergraduates {
        let person = entity("UndergraduateStudent", n);
        out.person(&person, "UndergraduateStudent", &department)?;
        if advised.contains(&n) {
            out.relation(
                &person,
                "advisor",
                &professors[draw.between(0, professors.len() - 1)],
            )?;
        }
        let count = draw.between(2, 4);
        for course in draw.sample(undergraduate_courses.len(), count) {
            out.relation(&person, "takesCourse", &undergraduate_courses[course])?;
        }
    }
    let ta_min = graduates.div_ceil(5);
    let ta_max = (graduates / 4).min(undergraduate_courses.len());
    if ta_min > ta_max {
        return Err(io::Error::other(
            "published TA fraction cannot fit distinct available courses",
        ));
    }
    let ta_count = draw.between(ta_min, ta_max);
    let ra_count = draw.between(graduates.div_ceil(4), graduates / 3);
    let teaching_students = draw.sample(graduates, ta_count);
    let teaching_courses = draw.sample(undergraduate_courses.len(), ta_count);
    let teaching: std::collections::BTreeMap<_, _> = teaching_students
        .into_iter()
        .zip(teaching_courses)
        .collect();
    let research: std::collections::BTreeSet<_> =
        draw.sample(graduates, ra_count).into_iter().collect();
    for n in 0..graduates {
        let person = entity("GraduateStudent", n);
        out.person(&person, "GraduateStudent", &department)?;
        out.degree(&person, "undergraduateDegreeFrom", draw)?;
        out.relation(
            &person,
            "advisor",
            &professors[draw.between(0, professors.len() - 1)],
        )?;
        let count = draw.between(1, 3);
        for course in draw.sample(graduate_courses.len(), count) {
            out.relation(&person, "takesCourse", &graduate_courses[course])?;
        }
        if let Some(course) = teaching.get(&n) {
            out.class(&person, "TeachingAssistant")?;
            out.relation(
                &person,
                "teachingAssistantOf",
                &undergraduate_courses[*course],
            )?;
        }
        if research.contains(&n) {
            out.class(&person, "ResearchAssistant")?;
            out.relation(&person, "worksFor", &department)?;
        }
        let count = draw.between(0, 5);
        for publication in draw.sample(professor_publications.len(), count) {
            out.relation(
                &professor_publications[publication],
                "publicationAuthor",
                &person,
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct BrokenWriter;
    impl Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("injected payload failure"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("injected flush failure"))
        }
    }
    #[test]
    fn actual_emitter_propagates_failed_writes() {
        let spec = Spec::new(
            0,
            0,
            1,
            "http://example.org/schema.owl".into(),
            "http://example.org/data/".into(),
        )
        .unwrap();
        let mut draw = Draw::new(0, 0);
        let error = department_document(&spec, 0, 0, &mut draw, &mut BrokenWriter).unwrap_err();
        assert_eq!(error.to_string(), "injected payload failure");
    }
}
