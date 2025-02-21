use rand::Rng;
use value::ConvexValue;

pub const DIMENSIONS: u32 = 4;

pub fn random_vector_value(rng: &mut impl Rng) -> ConvexValue {
    vector_to_value(random_vector(rng))
}

pub fn random_vector_with_dimens(rng: &mut impl Rng, dimensions: u32) -> Vec<f32> {
    (0..dimensions).map(|_| rng.random()).collect()
}

pub fn random_vector(rng: &mut impl Rng) -> Vec<f32> {
    random_vector_with_dimens(rng, DIMENSIONS)
}

pub fn vector_to_value(vector: Vec<f32>) -> ConvexValue {
    vector
        .into_iter()
        .map(|f| ConvexValue::Float64(f as f64))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
