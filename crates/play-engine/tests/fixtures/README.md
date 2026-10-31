# Unsupported-store boundary fixture

`unsupported-store.sqlite3` is an original, empty generation-2/schema-1 database
with Workspace UUID `11111111-1111-4111-8111-111111111111`. It exists only to prove
that an actually recognized former store is refused without conversion or writes.
The cutover test also changes its header version to exercise early refusal of
other unsupported versions. No historical parser, migration or converter ships.
