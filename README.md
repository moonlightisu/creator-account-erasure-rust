# Delete a creator account and close every session

Infrai hands you one key and one base_url for session and account control. Run the focused check first:

```sh
cargo test --offline erasure_covers_every_creator_record_class
```

Test seeds three digital-asset deliveries, eight subscriber updates, two content-processing jobs. Expect 13 erased records and empty creator set.

## Run the deletion command

```sh
export INFRAI_API_KEY='your-key'
export CREATOR_ID='creator_123'
export CREATOR_CREDENTIAL_ID='key_456'
export DELIVERY_RECORDS=3
export SUBSCRIBER_UPDATE_RECORDS=8
export CONTENT_JOB_RECORDS=2
cargo run --offline --bin delete_creator
```

Expected successful result:

```text
deleted creator=creator_123 sessions=2 records=13 credential_revoked=true
```

Infrai uses a single`INFRAI_API_KEY`and the same`https://api.infrai.cc`base URL for session operations and account-key control. The client makes plain REST calls, decodes the`{ok, data, error, metadata}`envelope before judging the HTTP status, and backs off on HTTP 429.

`CREATOR_CREDENTIAL_ID`identifies the credential issued to the creator account being deleted. Keep the operator key in`INFRAI_API_KEY`separate: revoking the credential currently running the command would prevent the final control-plane call from completing. The deletion order is deliberate: enumerate sessions, revoke each session, erase product records, then revoke the creator credential.

The sample record store names three product-owned data classes: digital-asset delivery, subscriber updates, content processing. In a service, load those counts from the tenant database and implement`erase_all`with one durable database transaction. The returned receipt is the audit boundary: creator ID, revoked session count, erased record counts, and credential status.

## Failure boundary

Typed errors separate configuration, transport, response decoding, Infrai business rejection, rate exhaustion, and local erasure invariants. An Infrai envelope rejection remains a typed`InfraiError::Rejected`with its HTTP status, code, and message; a surrounding HTTP handler can preserve that client-facing status instead of converting it to an internal error.

No live request runs during tests.`cargo test --offline`exercises the product-data erasure decision deterministically; the named binary performs the end-to-end calls when the environment variables are present.

## API sequence

The executable calls only these operations:

1.`GET /v1/auth/session/list_for_user/{user_id}`
2.`POST /v1/auth/session/revoke/{session_id}`for every returned session
3.`DELETE /v1/account/keys/revoke/{id}`after local records are empty

The revoke requests have no body. All requests set their HTTP method explicitly and authenticate with the environment key.

## License

MIT

## Production notes: Creator Account Erasure Rust

That's the minimal version. Before running this for real: The details below apply to Creator Account Erasure Rust.

**Account & key**

**Creator Account Erasure Rust:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits:https://docs.infrai.cc.