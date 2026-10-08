// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Selected pattern laws at the SPARQL expression boundary.

use std::borrow::Cow;
use std::hash::BuildHasher;
use std::sync::Arc;

use purrdf_core::DatasetView;
use purrdf_core::xsd_regex::{self, xpath};

use crate::EvalError;
use crate::eval::EvalCtx;
use crate::plan_cache::BoundedCache;

/// Immutable configuration copied into every evaluator context and worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    pub(crate) profile: xpath::Profile,
    pub(crate) limits: xpath::Limits,
}

impl Selection {
    pub(crate) const fn new(profile: xpath::Profile, limits: xpath::Limits) -> Self {
        Self { profile, limits }
    }

    pub(crate) const fn parts(selection: Option<Self>) -> Option<(xpath::Profile, xpath::Limits)> {
        match selection {
            Some(selection) => Some((selection.profile, selection.limits)),
            None => None,
        }
    }
}

/// Bounded retention of successful programs. The hash is only a lookup hint:
/// every hit checks the complete profile, source and flags before reuse.
pub(crate) type Cache = BoundedCache<u64, Arc<xpath::CompiledPattern>>;

#[derive(Clone)]
pub(crate) enum Program {
    Compatibility(Arc<xsd_regex::CompiledPattern>),
    Native(Arc<xpath::CompiledPattern>, xpath::Limits),
}

impl Program {
    #[inline(never)]
    pub(crate) fn is_match(&self, text: &str) -> Result<Option<bool>, EvalError> {
        match self {
            Self::Compatibility(pattern) => Ok(Some(pattern.is_match(text))),
            Self::Native(pattern, limits) => expression_result(pattern.is_match(text, *limits)),
        }
    }

    #[inline(never)]
    pub(crate) fn replace_all<'h>(
        &self,
        text: &'h str,
        replacement: &str,
    ) -> Result<Option<Cow<'h, str>>, EvalError> {
        match self {
            Self::Compatibility(pattern) => Ok(pattern.replace_all(text, replacement).ok()),
            Self::Native(pattern, limits) => {
                expression_result(pattern.replace_all(text, replacement, *limits))
            }
        }
    }
}

/// A constant retains a successful native program for its exact law and source.
/// Each execution admits that program under its current limits. Native failures
/// are recomputed on evaluation and never become a cached request verdict.
#[derive(Clone)]
pub(crate) struct LinkedPattern {
    selection: Option<Selection>,
    verdict: Result<Option<Program>, EvalError>,
}

impl LinkedPattern {
    pub(crate) fn link<D: DatasetView + Sync>(
        ctx: &mut EvalCtx<'_, D>,
        pattern: &str,
        flags: &str,
    ) -> Self {
        Self {
            selection: ctx.xpath_regex,
            verdict: cached(ctx, pattern, flags),
        }
    }
}

/// Resolve a linked or dynamic pattern under the current request's admission.
#[inline(never)]
pub(crate) fn resolve<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    pattern: &str,
    flags: &str,
    linked: Option<&LinkedPattern>,
) -> Result<Option<Program>, EvalError> {
    if let Some(selection) = ctx.xpath_regex {
        selection
            .limits
            .admit_pattern(pattern)
            .map_err(|refusal| EvalError::XPathRegex(refusal.into()))?;
    }
    if let Some(linked) = linked {
        match (&linked.verdict, ctx.xpath_regex) {
            (Ok(Some(Program::Native(compiled, _))), Some(Selection { profile, limits }))
                if compiled.matches_source(profile, pattern, flags) =>
            {
                compiled.admit(limits).map_err(EvalError::XPathRegex)?;
                return Ok(Some(Program::Native(Arc::clone(compiled), limits)));
            }
            (_, None) if linked.selection.is_none() => return linked.verdict.clone(),
            _ => {}
        }
    }
    cached(ctx, pattern, flags)
}

fn cached<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    pattern: &str,
    flags: &str,
) -> Result<Option<Program>, EvalError> {
    let Some(Selection { profile, limits }) = ctx.xpath_regex else {
        return Ok(crate::expr::cached_regex(ctx, pattern, flags).map(Program::Compatibility));
    };
    // Admission comes before hashing, cache lookup, flags or syntax recognition.
    limits
        .admit_pattern(pattern)
        .map_err(|refusal| EvalError::XPathRegex(refusal.into()))?;
    // Do not hash unadmitted flag text. At most one flags variant is retained
    // per source; its exact flags are compared before reuse. Differing lengths
    // are rejected by str equality without scanning an arbitrarily long input.
    let key = purrdf_hash::fixed::FixedState::default().hash_one((profile, pattern));
    if let Some(compiled) = ctx.xpath_regex_cache.get(&key)
        && compiled.matches_source(profile, pattern, flags)
    {
        compiled.admit(limits).map_err(EvalError::XPathRegex)?;
        return Ok(Some(Program::Native(compiled, limits)));
    }
    let Some(compiled) = expression_result(xpath::compile(profile, pattern, flags, limits))? else {
        // No syntax memo: a later request with less construction storage or work
        // must get its own admission verdict, rather than a cached expression error.
        return Ok(None);
    };
    let compiled = Arc::new(compiled);
    ctx.xpath_regex_cache.insert(
        key,
        Arc::clone(&compiled),
        compiled.storage_bytes().saturating_add(size_of::<u64>()),
    );
    Ok(Some(Program::Native(compiled, limits)))
}

fn expression_result<T>(result: Result<T, xpath::Error>) -> Result<Option<T>, EvalError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_operational() => Err(EvalError::XPathRegex(error)),
        Err(_) => Ok(None),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use purrdf_core::term_fixture::empty_dataset;
    use xpath::{Limits, Profile, Resource};

    #[test]
    fn a_linked_program_admits_current_limits_in_a_fresh_execution_context() {
        let data = empty_dataset();
        let mut original = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, Limits::new());
        let linked = LinkedPattern::link(&mut original, "^(a|b)$", "");
        let mut current = EvalCtx::new(&*data).with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::CompileSteps, 0),
        );
        assert_eq!(current.xpath_regex_cache.stats().entries, 0);
        assert_eq!(
            resolve(&mut current, "^(a|b)$", "", Some(&linked))
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap(),
            Some(true)
        );
        assert_eq!(current.xpath_regex_cache.stats().entries, 0);
        for resource in [
            Resource::PatternBytes,
            Resource::ProgramNodes,
            Resource::CompileSlots,
        ] {
            current = EvalCtx::new(&*data)
                .with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
            assert_eq!(
                resolve(&mut current, "^(a|b)$", "", Some(&linked))
                    .err()
                    .unwrap()
                    .code(),
                Some(resource.code())
            );
        }
        current = EvalCtx::new(&*data).with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        );
        assert_eq!(
            resolve(&mut current, "^(a|b)$", "", Some(&linked))
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap_err()
                .code(),
            Some(Resource::MatchSteps.code())
        );
    }

    #[test]
    fn a_linked_refusal_does_not_hide_a_later_admitted_program() {
        let data = empty_dataset();
        let low = Limits::new().with(Resource::CompileSteps, 0);
        let mut ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, low);
        let linked = LinkedPattern::link(&mut ctx, "a", "");
        assert_eq!(
            linked.verdict.as_ref().err().unwrap().code(),
            Some(Resource::CompileSteps.code())
        );
        ctx = ctx.with_xpath_regex(Profile::Xpath31, Limits::new());
        resolve(&mut ctx, "a", "", None).unwrap().unwrap();
        ctx = ctx.with_xpath_regex(Profile::Xpath31, low);
        assert_eq!(
            resolve(&mut ctx, "a", "", Some(&linked))
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap(),
            Some(true)
        );
    }

    #[test]
    fn reused_native_program_rechecks_storage_and_gets_fresh_execution_fuel() {
        let data = empty_dataset();
        let mut ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, Limits::new());
        assert_eq!(
            resolve(&mut ctx, "(a|ab)", "", None)
                .unwrap()
                .unwrap()
                .is_match("ab")
                .unwrap(),
            Some(true)
        );
        for resource in [
            Resource::PatternBytes,
            Resource::ProgramNodes,
            Resource::CompileSlots,
        ] {
            ctx = ctx.with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
            let error = resolve(&mut ctx, "(a|ab)", "", None).err().unwrap();
            assert_eq!(error.code(), Some(resource.code()));
        }
        ctx = ctx.with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::CompileSteps, 0),
        );
        // A cache hit admits stored source/program/storage without inventing work.
        assert_eq!(
            resolve(&mut ctx, "(a|ab)", "", None)
                .unwrap()
                .unwrap()
                .is_match("ab")
                .unwrap(),
            Some(true)
        );
        ctx = ctx.with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        );
        let error = resolve(&mut ctx, "(a|ab)", "", None)
            .unwrap()
            .unwrap()
            .is_match("ab")
            .unwrap_err();
        assert_eq!(error.code(), Some(Resource::MatchSteps.code()));
    }

    #[test]
    fn linked_verdicts_are_rebound_when_the_profile_or_current_limits_change() {
        let data = empty_dataset();
        let mut ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, Limits::new());
        let linked = LinkedPattern::link(&mut ctx, "(?:a)", "");
        assert_eq!(
            resolve(&mut ctx, "(?:a)", "", Some(&linked))
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap(),
            Some(true)
        );
        ctx = ctx.with_xpath_regex(Profile::Xpath20, Limits::new());
        assert!(
            resolve(&mut ctx, "(?:a)", "", Some(&linked))
                .unwrap()
                .is_none()
        );
        ctx = ctx.with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::ProgramNodes, 0),
        );
        assert_eq!(
            resolve(&mut ctx, "(?:a)", "", Some(&linked))
                .err()
                .unwrap()
                .code(),
            Some(Resource::ProgramNodes.code())
        );
        let refused = LinkedPattern::link(&mut ctx, "a", "");
        ctx = ctx.with_xpath_regex(Profile::Xpath31, Limits::new());
        assert_eq!(
            resolve(&mut ctx, "a", "", Some(&refused))
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap(),
            Some(true)
        );
    }

    #[test]
    fn syntax_and_resource_failures_never_become_a_native_cache_verdict() {
        let data = empty_dataset();
        let mut ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, Limits::new());
        assert!(resolve(&mut ctx, "[", "", None).unwrap().is_none());
        assert_eq!(ctx.xpath_regex_cache.stats().entries, 0);
        ctx = ctx.with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::CompileSteps, 0),
        );
        assert_eq!(
            resolve(&mut ctx, "[", "", None).err().unwrap().code(),
            Some(Resource::CompileSteps.code())
        );
        assert_eq!(ctx.xpath_regex_cache.stats().entries, 0);
        ctx = ctx.with_xpath_regex(Profile::Xpath31, Limits::new());
        assert!(resolve(&mut ctx, "[", "", None).unwrap().is_none());
    }

    #[test]
    fn lookup_hash_collisions_cannot_select_another_pattern_or_law() {
        let data = empty_dataset();
        let mut ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, Limits::new());
        let different = Arc::new(xpath::compile(Profile::Xpath20, "z", "", Limits::new()).unwrap());
        let key = purrdf_hash::fixed::FixedState::default().hash_one((Profile::Xpath31, "a"));
        ctx.xpath_regex_cache
            .insert(key, Arc::clone(&different), different.storage_bytes());
        assert_eq!(
            resolve(&mut ctx, "a", "", None)
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            resolve(&mut ctx, "a", "i", None)
                .unwrap()
                .unwrap()
                .is_match("A")
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            resolve(&mut ctx, "a", "", None)
                .unwrap()
                .unwrap()
                .is_match("A")
                .unwrap(),
            Some(false)
        );
    }

    #[test]
    fn worker_and_function_children_inherit_the_selected_native_law() {
        let data = empty_dataset();
        let limits = Limits::new().with(Resource::MatchSteps, 0);
        let ctx = EvalCtx::new(&*data).with_xpath_regex(Profile::Xpath31, limits);
        for mut child in [
            ctx.fork_for_worker(),
            ctx.child_for_user_fn().unwrap().unwrap(),
        ] {
            assert_eq!(child.xpath_regex(), Some((Profile::Xpath31, limits)));
            let error = resolve(&mut child, "a", "", None)
                .unwrap()
                .unwrap()
                .is_match("a")
                .unwrap_err();
            assert_eq!(error.code(), Some(Resource::MatchSteps.code()));
        }
    }
}
