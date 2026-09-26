// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stateful, model-based testing.
//!
//! A [`ReferenceStateMachine`] is a model: an initial state, the transitions
//! allowed from a state, and how a transition changes it. [`sequences`]
//! generates runs of that model — each transition drawn from the strategy the
//! *current* model state offers — and a [`SystemUnderTest`] replays a run
//! against the real implementation, checking it against the model after every
//! step.
//!
//! A run is generated from one choice sequence like any other value, so a
//! failing run shrinks like any other value: deleting a transition's span
//! drops the transition, and the transitions after it are regenerated from
//! their own choices under the new model state. Shrinking therefore lands on a
//! short run of simple transitions that still fails.
//!
//! ```
//! use purrdf_testkit::prop::prelude::*;
//! use purrdf_testkit::prop::state_machine::{self, ReferenceStateMachine, SystemUnderTest};
//!
//! #[derive(Debug, Clone)]
//! enum Op {
//!     Push(u8),
//!     Pop,
//! }
//!
//! #[derive(Debug)]
//! struct StackModel;
//!
//! impl ReferenceStateMachine for StackModel {
//!     type State = Vec<u8>;
//!     type Transition = Op;
//!
//!     fn init_state(&self) -> BoxedStrategy<Vec<u8>> {
//!         Just(Vec::new()).boxed()
//!     }
//!
//!     fn transitions(&self, state: &Vec<u8>) -> BoxedStrategy<Op> {
//!         if state.is_empty() {
//!             any::<u8>().prop_map(Op::Push).boxed()
//!         } else {
//!             prop_oneof![any::<u8>().prop_map(Op::Push), Just(Op::Pop)].boxed()
//!         }
//!     }
//!
//!     fn apply(&self, mut state: Vec<u8>, op: &Op) -> Vec<u8> {
//!         match op {
//!             Op::Push(value) => state.push(*value),
//!             Op::Pop => {
//!                 state.pop();
//!             }
//!         }
//!         state
//!     }
//! }
//!
//! struct RealStack(Vec<u8>);
//!
//! impl SystemUnderTest<StackModel> for RealStack {
//!     fn init(state: &Vec<u8>) -> Self {
//!         Self(state.clone())
//!     }
//!
//!     fn apply(&mut self, op: &Op, _after: &Vec<u8>) -> Result<(), TestCaseError> {
//!         match op {
//!             Op::Push(value) => self.0.push(*value),
//!             Op::Pop => {
//!                 self.0.pop();
//!             }
//!         }
//!         Ok(())
//!     }
//!
//!     fn check_invariants(&self, state: &Vec<u8>) -> Result<(), TestCaseError> {
//!         prop_assert_eq!(&self.0, state);
//!         Ok(())
//!     }
//! }
//!
//! purrdf_testkit::prop::run_test(
//!     &Config::with_cases(32),
//!     "stack_matches_its_model",
//!     &state_machine::sequences(StackModel, 0..20),
//!     |run| run.execute::<RealStack>(),
//! );
//! ```

use std::fmt;
use std::sync::Arc;

use super::choices::{Choices, Invalid};
use super::collection::{SizeRange, Sizer};
use super::runner::TestCaseError;
use super::strategy::{BoxedStrategy, Strategy};

/// The model a stateful test checks an implementation against.
pub trait ReferenceStateMachine {
    /// The model's state.
    type State: Clone + fmt::Debug;
    /// One operation.
    type Transition: Clone + fmt::Debug;

    /// The initial states.
    fn init_state(&self) -> BoxedStrategy<Self::State>;

    /// The transitions allowed from `state`.
    fn transitions(&self, state: &Self::State) -> BoxedStrategy<Self::Transition>;

    /// Whether `transition` may be applied in `state`. A generated transition
    /// that fails its precondition is redrawn and charged to the reject
    /// budget. The default admits everything.
    fn preconditions(&self, state: &Self::State, transition: &Self::Transition) -> bool {
        let _ = (state, transition);
        true
    }

    /// The state after `transition`.
    fn apply(&self, state: Self::State, transition: &Self::Transition) -> Self::State;
}

/// The implementation a [`ReferenceStateMachine`] models.
pub trait SystemUnderTest<M: ReferenceStateMachine>: Sized {
    /// The implementation in the model's initial `state`.
    fn init(state: &M::State) -> Self;

    /// Apply `transition`; `after` is the model's state once it has been
    /// applied there. Return a failure if the implementation disagrees.
    fn apply(&mut self, transition: &M::Transition, after: &M::State) -> Result<(), TestCaseError>;

    /// Check the implementation against the model's `state`, after
    /// initialisation and after every transition. The default checks nothing.
    fn check_invariants(&self, state: &M::State) -> Result<(), TestCaseError> {
        let _ = state;
        Ok(())
    }
}

/// Runs of `model` with a number of transitions in `steps`.
pub fn sequences<M: ReferenceStateMachine>(model: M, steps: impl Into<SizeRange>) -> Sequences<M> {
    Sequences {
        model: Arc::new(model),
        steps: steps.into(),
    }
}

/// See [`sequences`].
pub struct Sequences<M> {
    model: Arc<M>,
    steps: SizeRange,
}

impl<M> Clone for Sequences<M> {
    fn clone(&self) -> Self {
        Self {
            model: Arc::clone(&self.model),
            steps: self.steps,
        }
    }
}

impl<M> fmt::Debug for Sequences<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sequences")
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

/// One generated run: an initial model state and the transitions after it.
pub struct Sequence<M: ReferenceStateMachine> {
    model: Arc<M>,
    /// The initial model state.
    pub initial: M::State,
    /// The transitions, in order.
    pub transitions: Vec<M::Transition>,
}

impl<M: ReferenceStateMachine> fmt::Debug for Sequence<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sequence")
            .field("initial", &self.initial)
            .field("transitions", &self.transitions)
            .finish_non_exhaustive()
    }
}

impl<M: ReferenceStateMachine> Sequence<M> {
    /// Replay the run against `S`, checking its invariants after
    /// initialisation and after every transition. A failure names the step.
    pub fn execute<S: SystemUnderTest<M>>(&self) -> Result<(), TestCaseError> {
        let mut state = self.initial.clone();
        let mut system = S::init(&state);
        system
            .check_invariants(&state)
            .map_err(|error| at_step(error, "after initialisation"))?;
        for (step, transition) in self.transitions.iter().enumerate() {
            state = self.model.apply(state, transition);
            let place = || format!("at step {step} ({transition:?})");
            system
                .apply(transition, &state)
                .map_err(|error| at_step(error, &place()))?;
            system
                .check_invariants(&state)
                .map_err(|error| at_step(error, &place()))?;
        }
        Ok(())
    }
}

fn at_step(error: TestCaseError, place: &str) -> TestCaseError {
    match error {
        TestCaseError::Fail(message) => TestCaseError::Fail(format!("{place}: {message}")),
        reject @ TestCaseError::Reject(_) => reject,
    }
}

impl<M: ReferenceStateMachine> Strategy for Sequences<M> {
    type Value = Sequence<M>;

    fn generate(&self, choices: &mut Choices) -> Result<Sequence<M>, Invalid> {
        let initial = self.model.init_state().generate(choices)?;
        let mut state = initial.clone();
        let mut transitions = Vec::new();
        let sizer = Sizer::start(choices, self.steps);
        while sizer.more(choices, transitions.len(), false)? {
            let offered = self.model.transitions(&state);
            let transition = loop {
                let transition = offered.generate(choices)?;
                if self.model.preconditions(&state, &transition) {
                    break transition;
                }
                choices.reject("state_machine: a transition failed its preconditions")?;
            };
            state = self.model.apply(state, &transition);
            transitions.push(transition);
        }
        Ok(Sequence {
            model: Arc::clone(&self.model),
            initial,
            transitions,
        })
    }
}
