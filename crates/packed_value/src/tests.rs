use cmd_util::env::env_config;
use proptest::prelude::*;
use value::{
    sorting::write_sort_key,
    ConvexValue,
};

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
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
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

    #[test]
    fn test_sort_key_roundtrips(v in any::<ConvexValue>()) {
        let packed_value = PackedValue::pack(&v);
        let mut sort_key = vec![];
        write_sort_key(packed_value.as_ref().open().unwrap(), &mut sort_key).unwrap();
        assert_eq!(ConvexValue::read_sort_key(&mut &sort_key[..], ).unwrap(), v);
    }
}
