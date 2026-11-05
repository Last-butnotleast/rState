use rstate::{JsonCodec, Persisted, SnapshotCodec};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Context {
    label: String,
    count: u64,
}

#[test]
fn public_envelopes_round_trip_and_share_the_existing_json_codec_contract() {
    let persisted = Persisted {
        context: Context {
            label: "queued".into(),
            count: 4,
        },
        version: 2,
    };
    let bytes = serde_json::to_vec(&persisted).unwrap();
    assert_eq!(
        std::str::from_utf8(&bytes).unwrap(),
        "{\"context\":{\"label\":\"queued\",\"count\":4},\"version\":2}"
    );
    let decoded: Persisted<Context> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.context, persisted.context);
    assert_eq!(decoded.version, 2);

    let codec = JsonCodec::<Context>::new();
    assert_eq!(codec.encode(&persisted.context, 2).unwrap(), bytes);
    let restored = codec.decode(&bytes).unwrap().into_snapshot(2).unwrap();
    assert_eq!(*restored.context, persisted.context);
}

#[test]
fn borrowed_contexts_serialize_without_cloning_or_serializing_snapshot_metadata() {
    let context = Context {
        label: "borrowed".into(),
        count: 8,
    };
    let encoded = serde_json::to_vec(&Persisted {
        context: &context,
        version: 6,
    })
    .unwrap();
    let restored: Persisted<Context> = serde_json::from_slice(&encoded).unwrap();
    let snapshot = restored.into_snapshot(6).unwrap();

    assert_eq!(*snapshot.context, context);
    assert_eq!(snapshot.history_status().limit, None);
    assert_eq!(snapshot.history_status().past, 0);
    assert_eq!(snapshot.history_status().future, 0);
}
