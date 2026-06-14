# rust-avro

Rust utilities for working with Apache Avro — CSV-to-Avro conversion and related tooling.

## Performance

Writing Avro records via the generic `Record` API (`apache_avro`) is relatively slow, even in Rust. 
The generic record construction path adds overhead per record.

An optimized custom C language module processing the same test data achieves approximately **10x faster** throughput compared to the Rust `Record`-based approach.
