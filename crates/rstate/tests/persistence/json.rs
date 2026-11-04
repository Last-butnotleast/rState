use rstate::{JsonCodec, SnapshotCodec};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Context {
    count: u64,
    label: String,
}

#[test]
fn optional_json_codec_round_trips_a_versioned_full_context_and_preserves_decode_errors() {
    let codec = JsonCodec::<Context>::new();
    let context = Context {
        count: 12,
        label: "loaded".into(),
    };
    let encoded = codec.encode(&context, 7).unwrap();
    assert_eq!(
        std::str::from_utf8(&encoded).unwrap(),
        "{\"context\":{\"count\":12,\"label\":\"loaded\"},\"version\":7}"
    );
    let restored = codec.decode(&encoded).unwrap();
    assert_eq!(restored.context, context);
    assert_eq!(restored.version, 7);
    let error = codec.decode(b"invalid").unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(format!("{codec:?}"), "JsonCodec");
}
