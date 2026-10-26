CREATE TABLE candidates (
    challenge_key TEXT NOT NULL REFERENCES challenges(key),
    stage_key TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    result_id TEXT NOT NULL REFERENCES saved_results(id),
    record_json TEXT NOT NULL CHECK(length(CAST(record_json AS BLOB))<=16384),
    created_ms INTEGER NOT NULL,
    PRIMARY KEY(challenge_key,stage_key,revision)
);
