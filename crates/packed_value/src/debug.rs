use std::fmt;

use flexbuffers::Buffer;
use value::ConvexValue;

use super::{
    OpenedObject,
    OpenedValue,
    PackedValue,
};

impl<B: Buffer> fmt::Debug for PackedValue<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match ConvexValue::try_from(self.clone()) {
            Ok(v) => write!(f, "{v:?}"),
            Err(e) => write!(f, "OpenedValue(invalid: {e}"),
        }
    }
}

impl<B: Buffer> fmt::Display for PackedValue<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match ConvexValue::try_from(self.clone()) {
            Ok(v) => write!(f, "{v}"),
            Err(e) => write!(f, "OpenedValue(invalid: {e}"),
        }
    }
}

impl<B: Buffer> fmt::Debug for OpenedValue<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match ConvexValue::try_from(self.clone()) {
            Ok(v) => write!(f, "{v:?}"),
            Err(e) => write!(f, "OpenedValue(invalid: {e}"),
        }
    }
}

impl<B: Buffer> fmt::Display for OpenedValue<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match ConvexValue::try_from(self.clone()) {
            Ok(v) => write!(f, "{v}"),
            Err(e) => write!(f, "OpenedValue(invalid: {e}"),
        }
    }
}

impl<B: Buffer> fmt::Debug for OpenedObject<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", OpenedValue::Object(self.clone()))
    }
}

impl<B: Buffer> fmt::Display for OpenedObject<B>
where
    B::BufferString: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", OpenedValue::Object(self.clone()))
    }
}
