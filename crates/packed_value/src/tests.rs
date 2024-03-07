use proptest::prelude::*;
use value::ConvexValue;

use super::{
    ByteBuffer,
    OpenedValue,
    PackedValue,
};

fn roundtrip_test(v: ConvexValue) {
    let buf = PackedValue::pack(&v);
    let packed = buf.open().unwrap();
    let v2 = ConvexValue::try_from(packed).unwrap();
    assert_eq!(v, v2);
}

fn clone_test(v: OpenedValue<ByteBuffer>) -> anyhow::Result<()> {
    match v {
        OpenedValue::Array(ref a) => {
            for value_r in a.iter() {
                clone_test(value_r?)?;
            }
        },
        OpenedValue::Set(ref s) => {
            for value_r in s.iter() {
                clone_test(value_r?)?;
            }
        },
        OpenedValue::Map(ref m) => {
            for r in m.iter() {
                let (k, v) = r?;
                clone_test(k)?;
                clone_test(v)?;
            }
        },
        OpenedValue::Object(ref o) => {
            for r in o.iter() {
                let (_, v) = r?;
                clone_test(v)?;
            }
        },
        _ => (),
    }
    let cloned = v.clone();
    assert_eq!(ConvexValue::try_from(v)?, ConvexValue::try_from(cloned)?);
    Ok(())
}

proptest! {
    #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

    #[test]
    fn proptest_roundtrip(v in any::<ConvexValue>()) {
        roundtrip_test(v);
    }

    #[test]
    fn proptest_clone(v in any::<ConvexValue>()) {
        let p = PackedValue::pack(&v).open().unwrap();
        clone_test(p).unwrap();
    }
}
