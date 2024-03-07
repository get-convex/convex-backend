use std::{
    cmp::Ordering,
    marker::PhantomData,
};

use super::{
    config::ShapeConfig,
    ShapeEnum,
};
use crate::CountedShapeEnum;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Float64Shape<C: ShapeConfig> {
    _cfg: PhantomData<C>,
}

impl<C: ShapeConfig> Float64Shape<C> {
    pub fn shape_of(f: f64) -> CountedShapeEnum<C> {
        if f64::is_nan(f) {
            return ShapeEnum::NaN;
        }
        if f == f64::INFINITY {
            return ShapeEnum::PositiveInf;
        }
        if f == f64::NEG_INFINITY {
            return ShapeEnum::NegativeInf;
        }
        if matches!(f.total_cmp(&-0.0), Ordering::Equal) {
            return ShapeEnum::NegativeZero;
        }
        ShapeEnum::NormalFloat64
    }
}
