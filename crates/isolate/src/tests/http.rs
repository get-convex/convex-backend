use common::{
    errors::JsError,
    testing::{
        assert_roundtrips,
        TestPersistence,
    },
};
use headers::HeaderMap;
use http::Method;
use keybroker::Identity;
use must_let::must_let;
use proptest::prelude::*;
use runtime::testing::TestRuntime;
use serde_json::{
    json,
    Value as JsonValue,
};
use url::Url;

use crate::{
    http::{
        HttpRequest,
        HttpResponse,
    },
    test_helpers::UdfTest,
    IsolateConfig,
};

proptest! {
    #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

    #[test]
    fn test_http_request_roundtrip(v in any::<HttpRequest>()) {
        assert_roundtrips::<HttpRequest, JsonValue>(v);
    }

    #[test]
    fn test_http_response_roundtrip(v in any::<HttpResponse>()) {
        assert_roundtrips::<HttpResponse, JsonValue>(v);
    }
}
