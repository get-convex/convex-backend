use std::fmt;

/// PII is a light wrapper struct that implements Debug by omitting the contents
/// when printing in prod.
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct PII<T>(pub T);

impl<T> std::ops::Deref for PII<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> PII<T> {
    pub fn into_value(self) -> T {
        self.0
    }
}

impl<T> From<T> for PII<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

#[cfg(not(any(test, feature = "testing")))]
impl<T> fmt::Debug for PII<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PII({})", std::any::type_name::<T>())
    }
}

#[cfg(any(test, feature = "testing"))]
impl<T: fmt::Debug> fmt::Debug for PII<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PII({:?})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use value::{
        assert_obj,
        ConvexObject,
    };

    use super::PII;

    #[test]
    fn test_pii_debug_by_ref() -> anyhow::Result<()> {
        let obj: ConvexObject = assert_obj!("ssn" => 123456789);
        let pii_obj = PII(obj);
        let debug_pii = format!("{:?}", &pii_obj);
        // This is a test, so we print the PII wrapped in PII().
        // In particular this does not use the impl Deref to print the object directly.
        assert!(debug_pii.starts_with("PII("));
        assert!(debug_pii.ends_with(')'));
        Ok(())
    }
}
