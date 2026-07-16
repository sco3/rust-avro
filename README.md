# rust-avro

Rust utilities for working with Apache Avro — CSV-to-Avro conversion and related tooling.

## Performance

Writing Avro records via the generic `Record` API (`apache_avro`) is relatively slow, even in Rust.
The generic record construction path adds overhead per record.

### Benchmark: Custom Binary Writer vs `apache_avro::Writer`

Test data: 1,012,420 records, 97 fields each (629 MB CSV).

| Implementation | Time (avg 3 runs) | Throughput | Speedup |
|---|---|---|---|
| `main` — `apache_avro::Writer` + `Record::put` | ~32.3s | ~31K rec/s | 1x |
| `no-record` — custom binary writer (no `Record`) | ~3.2s | ~320K rec/s | **~10x** |

The `no-record` branch bypasses the `apache_avro::Writer` and `Record` abstraction entirely,
writing Avro binary fields directly to the output buffer. This eliminates per-record
heap allocation, schema validation, and BTreeMap lookups.

Test:
```
Converted 1012420 records from 'data/000000_0.csv' to '0.avro' in 3.216s (314764 records/s)
Converted 1012420 records from 'data/000000_0.csv' to '0.generic.avro' in 28.571s (35435 records/s). Generic record mode

```