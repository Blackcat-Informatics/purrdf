// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One smooth geodetic-normal equation, evaluated on intervals or Taylor jets.

use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, TaylorJet, TaylorWorkspace};

pub(crate) type NormalImage<T> = ([T; 3], [T; 3]);

pub(crate) trait NormalArithmetic {
    type Value: Clone;
    fn one(&mut self) -> Result<Self::Value, MathError>;
    fn mul(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError>;
    fn square(&mut self, a: &Self::Value) -> Result<Self::Value, MathError>;
    fn add(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError>;
    fn sub(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError>;
    fn div(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError>;
    fn neg(&mut self, a: &Self::Value) -> Result<Self::Value, MathError>;
    fn sqrt(&mut self, a: &Self::Value) -> Result<Self::Value, MathError>;
    fn sin_cos(&mut self, a: &Self::Value) -> Result<(Self::Value, Self::Value), MathError>;
}

#[derive(Debug)]
pub(crate) struct NormalArguments<T> {
    pub(crate) c: T,
    pub(crate) e2: T,
    pub(crate) sin0: T,
    pub(crate) phase: T,
    pub(crate) sine: T,
    pub(crate) cosine: T,
    pub(crate) vertical: T,
    pub(crate) horizontal: T,
    pub(crate) h: T,
    pub(crate) dsigma: T,
}

pub(crate) fn phase_speed<A: NormalArithmetic>(
    c: &A::Value,
    e2: &A::Value,
    sin0: &A::Value,
    h: &A::Value,
    dsigma: &A::Value,
    arithmetic: &mut A,
) -> Result<A::Value, MathError> {
    let e_sin0 = arithmetic.mul(e2, sin0)?;
    let numerator = arithmetic.neg(&e_sin0)?;
    let c_h = arithmetic.mul(c, h)?;
    let one = arithmetic.one()?;
    let denominator = arithmetic.add(&one, &c_h)?;
    let quotient = arithmetic.div(&numerator, &denominator)?;
    arithmetic.mul(&quotient, dsigma)
}

pub(crate) fn evaluate<A: NormalArithmetic>(
    args: &NormalArguments<A::Value>,
    arithmetic: &mut A,
) -> Result<NormalImage<A::Value>, MathError> {
    let (sin, cos) = arithmetic.sin_cos(&args.phase)?;
    let v = [
        arithmetic.mul(&args.c, &args.cosine)?,
        {
            let c_sin0 = arithmetic.mul(&args.c, &args.sin0)?;
            arithmetic.mul(&c_sin0, &args.sine)?
        },
        args.vertical.clone(),
    ];
    // |V|²=c²+e²*V_z² >=c², smooth and positive across coordinate poles.
    let norm2 = {
        let c2 = arithmetic.square(&args.c)?;
        let vertical2 = arithmetic.square(&args.vertical)?;
        let second = arithmetic.mul(&args.e2, &vertical2)?;
        arithmetic.add(&c2, &second)?
    };
    let norm = arithmetic.sqrt(&norm2)?;
    let unit = [
        arithmetic.div(&v[0], &norm)?,
        arithmetic.div(&v[1], &norm)?,
        arithmetic.div(&v[2], &norm)?,
    ];
    let normal = [
        {
            let x = arithmetic.mul(&cos, &unit[0])?;
            let y = arithmetic.mul(&sin, &unit[1])?;
            arithmetic.sub(&x, &y)?
        },
        {
            let x = arithmetic.mul(&sin, &unit[0])?;
            let y = arithmetic.mul(&cos, &unit[1])?;
            arithmetic.add(&x, &y)?
        },
        unit[2].clone(),
    ];
    let phase_derivative = phase_speed(
        &args.c,
        &args.e2,
        &args.sin0,
        &args.h,
        &args.dsigma,
        arithmetic,
    )?;
    let dv = [
        {
            let product = arithmetic.mul(&args.c, &args.sine)?;
            arithmetic.neg(&product)?
        },
        {
            let c_sin0 = arithmetic.mul(&args.c, &args.sin0)?;
            arithmetic.mul(&c_sin0, &args.cosine)?
        },
        args.horizontal.clone(),
    ];
    let log_norm_derivative = {
        let first = arithmetic.mul(&args.e2, &args.vertical)?;
        let product = arithmetic.mul(&first, &args.horizontal)?;
        arithmetic.div(&product, &norm2)?
    };
    let mut differentiate = |i: usize| {
        let first = arithmetic.div(&dv[i], &norm)?;
        let second = arithmetic.mul(&unit[i], &log_norm_derivative)?;
        let difference = arithmetic.sub(&first, &second)?;
        arithmetic.mul(&difference, &args.dsigma)
    };
    let dunit = [differentiate(0)?, differentiate(1)?, differentiate(2)?];
    let derivative = [
        {
            let first = arithmetic.mul(&cos, &dunit[0])?;
            let second = arithmetic.mul(&sin, &dunit[1])?;
            let difference = arithmetic.sub(&first, &second)?;
            let rotation = arithmetic.mul(&phase_derivative, &normal[1])?;
            arithmetic.sub(&difference, &rotation)?
        },
        {
            let first = arithmetic.mul(&sin, &dunit[0])?;
            let second = arithmetic.mul(&cos, &dunit[1])?;
            let sum = arithmetic.add(&first, &second)?;
            let rotation = arithmetic.mul(&phase_derivative, &normal[0])?;
            arithmetic.add(&sum, &rotation)?
        },
        dunit[2].clone(),
    ];
    Ok((normal, derivative))
}

#[derive(Debug)]
pub(crate) struct FixedArithmetic<'a>(pub(crate) &'a mut CoordinateMath);

impl NormalArithmetic for FixedArithmetic<'_> {
    type Value = FixedInterval;
    fn one(&mut self) -> Result<Self::Value, MathError> {
        FixedInterval::from_i64(1, self.0)
    }
    fn mul(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        a.mul(b, self.0)
    }
    fn square(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        a.square(self.0)
    }
    fn add(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        a.add(b, self.0)
    }
    fn sub(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        a.sub(b, self.0)
    }
    fn div(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        a.div(b, self.0)
    }
    fn neg(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        a.neg(self.0)
    }
    fn sqrt(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        a.sqrt(self.0)
    }
    fn sin_cos(&mut self, a: &Self::Value) -> Result<(Self::Value, Self::Value), MathError> {
        a.sin_cos_range(self.0)
    }
}

#[derive(Debug)]
pub(crate) struct TaylorArithmetic<'a, 'scope> {
    pub(crate) workspace: &'a mut TaylorWorkspace<'scope>,
    pub(crate) math: &'a mut CoordinateMath,
}

impl<'scope> NormalArithmetic for TaylorArithmetic<'_, 'scope> {
    type Value = TaylorJet<'scope>;
    fn one(&mut self) -> Result<Self::Value, MathError> {
        let one = FixedInterval::from_i64(1, self.math)?;
        self.workspace.constant(one, self.math)
    }
    fn mul(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.mul(*a, *b, self.math)
    }
    fn square(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.mul(*a, *a, self.math)
    }
    fn add(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.add(*a, *b, self.math)
    }
    fn sub(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.sub(*a, *b, self.math)
    }
    fn div(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.div(*a, *b, self.math)
    }
    fn neg(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        let minus_one = FixedInterval::from_i64(-1, self.math)?;
        self.workspace.scale(*a, &minus_one, self.math)
    }
    fn sqrt(&mut self, a: &Self::Value) -> Result<Self::Value, MathError> {
        self.workspace.sqrt(*a, self.math)
    }
    fn sin_cos(&mut self, a: &Self::Value) -> Result<(Self::Value, Self::Value), MathError> {
        self.workspace.sin_cos(*a, self.math)
    }
}
