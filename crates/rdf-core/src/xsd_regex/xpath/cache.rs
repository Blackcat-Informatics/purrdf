// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded successful-program reuse for validation facets.

use super::{CompiledPattern, Error, Limits, Profile, compile};

/// Retain at most one successful native pattern, without retaining failures.
///
/// This small cache owns no lookup keys or unbounded table. The retained program
/// carries its exact dated profile, source and flags. Every request admits the
/// source before lookup and rechecks the stored program's construction contract;
/// matching still requires that request's fresh finite limits.
#[derive(Debug, Default)]
pub struct PatternCache {
    program: Option<CompiledPattern>,
}

impl PatternCache {
    /// Compile or reuse the exact requested program under current admission.
    ///
    /// A failed compilation leaves the prior successful program intact. Skipped
    /// compilation consumes no compiler fuel, but source/program/storage bounds
    /// still apply to every hit. Neither language nor operational failures are
    /// cached as verdicts.
    ///
    /// # Errors
    /// Returns the native compiler's distinct language or operational cause.
    pub fn compiled(
        &mut self,
        profile: Profile,
        pattern: &str,
        flags: &str,
        limits: Limits,
    ) -> Result<&CompiledPattern, Error> {
        limits.admit_pattern(pattern)?;
        if let Some(program) = &self.program
            && program.matches_source(profile, pattern, flags)
        {
            program.admit(limits)?;
        } else {
            self.program = Some(compile(profile, pattern, flags, limits)?);
        }
        Ok(self
            .program
            .as_ref()
            .expect("successful admission stores a program"))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::xsd_regex::xpath::Resource;

    #[test]
    fn current_admission_precedes_reuse_and_failed_laws_do_not_poison_a_program() {
        let mut cache = PatternCache::default();
        assert!(
            cache
                .compiled(Profile::Xpath31, "(?:a)", "", Limits::new())
                .unwrap()
                .is_match("a", Limits::new())
                .unwrap()
        );
        for resource in [
            Resource::PatternBytes,
            Resource::ProgramNodes,
            Resource::CompileSlots,
        ] {
            assert!(
                matches!(cache.compiled(Profile::Xpath31, "(?:a)", "", Limits::new().with(resource, 0)), Err(Error::Resource(cause)) if cause.resource == resource)
            );
        }
        assert!(matches!(
            cache.compiled(Profile::Xpath20, "(?:a)", "", Limits::new()),
            Err(Error::Syntax { .. })
        ));
        assert!(
            cache
                .compiled(
                    Profile::Xpath31,
                    "(?:a)",
                    "",
                    Limits::new().with(Resource::CompileSteps, 0)
                )
                .unwrap()
                .is_match("a", Limits::new())
                .unwrap()
        );
        assert!(
            matches!(cache.compiled(Profile::Xpath31, "(?:a)", "i", Limits::new().with(Resource::CompileSteps, 0)), Err(Error::Resource(cause)) if cause.resource == Resource::CompileSteps)
        );
    }

    #[test]
    fn syntax_is_recomputed_under_current_compiler_bounds() {
        let mut cache = PatternCache::default();
        assert!(matches!(
            cache.compiled(Profile::Xpath31, "[", "", Limits::new()),
            Err(Error::Syntax { .. })
        ));
        assert!(
            matches!(cache.compiled(Profile::Xpath31, "[", "", Limits::new().with(Resource::CompileSteps, 0)), Err(Error::Resource(cause)) if cause.resource == Resource::CompileSteps)
        );
        assert!(cache.program.is_none());
    }
}
