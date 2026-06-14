use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

use apache_avro::schema::RecordField;
use apache_avro::Schema;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "avro_write", about = "Convert pipe-separated CSV to Avro format")]
struct Args {
    #[arg(short, long)]
    input: String,
    #[arg(short, long)]
    output: String,
    #[arg(short, long)]
    schema: String,
}

#[derive(Clone, Copy, Debug)]
enum FieldType {
    String,
    Long,
    Double,
    Int,
    Boolean,
    Float,
}

const BLOCK_SIZE: usize = 16000;

struct AvroWriter {
    buf: Vec<u8>,
    out: BufWriter<File>,
    marker: [u8; 16],
    num_values: usize,
}

impl AvroWriter {
    fn new(schema_json: &str, out: File) -> Self {
        let mut marker = [0u8; 16];
        {
            let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
            std::io::Read::read_exact(&mut f, &mut marker).expect("Failed to read random bytes");
        }

        let mut w = Self {
            buf: Vec::with_capacity(BLOCK_SIZE * 256),
            out: BufWriter::with_capacity(1 << 20, out),
            marker,
            num_values: 0,
        };
        w.write_header(schema_json);
        w
    }

    fn write_header(&mut self, schema_json: &str) {
        self.out.write_all(b"Obj\x01").unwrap();

        let schema_bytes = schema_json.as_bytes();

        let meta_entries = 2u64;
        encode_long(meta_entries as i64, &mut self.out);

        encode_string(b"avro.schema", &mut self.out);
        encode_bytes(schema_bytes, &mut self.out);

        encode_string(b"avro.codec", &mut self.out);
        encode_bytes(b"null", &mut self.out);

        encode_long(0, &mut self.out);
        self.out.write_all(&self.marker).unwrap();
    }

    fn write_union_null(&mut self) {
        encode_long(0, &mut self.buf);
    }

    fn write_union_value(&mut self, index: i64) {
        encode_long(index, &mut self.buf);
    }

    fn write_string(&mut self, s: &str) {
        encode_string(s.as_bytes(), &mut self.buf);
    }

    fn write_long(&mut self, v: i64) {
        encode_long(v, &mut self.buf);
    }

    fn write_int(&mut self, v: i32) {
        encode_long(v as i64, &mut self.buf);
    }

    fn write_double(&mut self, v: f64) {
        self.buf.write_all(&v.to_le_bytes()).unwrap();
    }

    fn write_float(&mut self, v: f32) {
        self.buf.write_all(&v.to_le_bytes()).unwrap();
    }

    fn write_boolean(&mut self, v: bool) {
        self.buf.write_all(&[v as u8]).unwrap();
    }

    fn write_field(&mut self, raw: &str, field_type: FieldType, nullable: bool) {
        if raw == "\\N" {
            assert!(
                nullable,
                "Null marker '\\N' found in non-nullable field"
            );
            self.write_union_null();
            return;
        }
        if nullable {
            self.write_union_value(1);
        }
        match field_type {
            FieldType::String => self.write_string(raw),
            FieldType::Long => {
                let v: i64 = raw.parse().unwrap_or_else(|e| panic!("Failed to parse '{}' as long: {}", raw, e));
                self.write_long(v);
            }
            FieldType::Double => {
                let v: f64 = raw.parse().unwrap_or_else(|e| panic!("Failed to parse '{}' as double: {}", raw, e));
                self.write_double(v);
            }
            FieldType::Int => {
                let v: i32 = raw.parse().unwrap_or_else(|e| panic!("Failed to parse '{}' as int: {}", raw, e));
                self.write_int(v);
            }
            FieldType::Boolean => {
                let v: bool = raw.parse().unwrap_or_else(|e| panic!("Failed to parse '{}' as boolean: {}", raw, e));
                self.write_boolean(v);
            }
            FieldType::Float => {
                let v: f32 = raw.parse().unwrap_or_else(|e| panic!("Failed to parse '{}' as float: {}", raw, e));
                self.write_float(v);
            }
        }
    }

    fn flush_block(&mut self) {
        if self.num_values == 0 {
            return;
        }
        let block_len = self.buf.len();
        encode_long(self.num_values as i64, &mut self.out);
        encode_long(block_len as i64, &mut self.out);
        self.out.write_all(&self.buf).unwrap();
        self.out.write_all(&self.marker).unwrap();
        self.buf.clear();
        self.num_values = 0;
    }

    fn finish(mut self) {
        self.flush_block();
        self.out.flush().unwrap();
    }
}

fn encode_varint<W: Write>(mut z: u64, w: &mut W) {
    loop {
        if z <= 0x7F {
            w.write_all(&[z as u8]).unwrap();
            break;
        } else {
            w.write_all(&[0x80 | (z & 0x7F) as u8]).unwrap();
            z >>= 7;
        }
    }
}

fn encode_long<W: Write>(n: i64, w: &mut W) {
    let z = ((n << 1) ^ (n >> 63)) as u64;
    encode_varint(z, w);
}

fn encode_bytes<W: Write>(b: &[u8], w: &mut W) {
    encode_long(b.len() as i64, w);
    w.write_all(b).unwrap();
}

fn encode_string<W: Write>(s: &[u8], w: &mut W) {
    encode_bytes(s, w);
}

#[derive(Clone, Copy, Debug)]
struct FieldInfo {
    field_type: FieldType,
    nullable: bool,
}

fn extract_field_info(field: &RecordField) -> FieldInfo {
    match &field.schema {
        Schema::Union(u) => {
            let variants = u.variants();
            if variants.len() == 2 && matches!(variants[0], Schema::Null) {
                let ft = match &variants[1] {
                    Schema::String => FieldType::String,
                    Schema::Long => FieldType::Long,
                    Schema::Double => FieldType::Double,
                    Schema::Int => FieldType::Int,
                    Schema::Boolean => FieldType::Boolean,
                    Schema::Float => FieldType::Float,
                    _ => FieldType::String,
                };
                return FieldInfo { field_type: ft, nullable: true };
            }
            FieldInfo { field_type: FieldType::String, nullable: false }
        }
        Schema::String => FieldInfo { field_type: FieldType::String, nullable: false },
        Schema::Long => FieldInfo { field_type: FieldType::Long, nullable: false },
        Schema::Double => FieldInfo { field_type: FieldType::Double, nullable: false },
        Schema::Int => FieldInfo { field_type: FieldType::Int, nullable: false },
        Schema::Boolean => FieldInfo { field_type: FieldType::Boolean, nullable: false },
        Schema::Float => FieldInfo { field_type: FieldType::Float, nullable: false },
        _ => FieldInfo { field_type: FieldType::String, nullable: false },
    }
}

fn main() {
    let args = Args::parse();

    let schema_str = std::fs::read_to_string(&args.schema)
        .unwrap_or_else(|e| panic!("Failed to read schema file '{}': {}", args.schema, e));
    let schema = Schema::parse_str(&schema_str)
        .unwrap_or_else(|e| panic!("Failed to parse schema: {}", e));

    let record_schema = match &schema {
        Schema::Record(rs) => rs,
        _ => panic!("Schema must be a record type"),
    };

    let field_infos: Vec<FieldInfo> = record_schema.fields.iter().map(extract_field_info).collect();
    let num_fields = field_infos.len();

    let csv_file = File::open(&args.input)
        .unwrap_or_else(|e| panic!("Failed to open input file '{}': {}", args.input, e));
    let reader = BufReader::new(csv_file);

    let out_file = File::create(&args.output)
        .unwrap_or_else(|e| panic!("Failed to create output file '{}': {}", args.output, e));

    let mut writer = AvroWriter::new(&schema_str, out_file);

    let mut line_count: u64 = 0;
    let mut written: u64 = 0;

    let start = Instant::now();

    for line_result in reader.lines() {
        let line = line_result.expect("Failed to read line");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let columns: Vec<&str> = trimmed.split('|').collect();
        if columns.len() < num_fields {
            eprintln!(
                "Warning: line {} has {} columns, expected {}. Skipping.",
                line_count,
                columns.len(),
                num_fields
            );
            line_count += 1;
            continue;
        }

        for i in 0..num_fields {
            let fi = field_infos[i];
            writer.write_field(columns[i], fi.field_type, fi.nullable);
        }
        writer.num_values += 1;

        if writer.buf.len() >= BLOCK_SIZE * 256 {
            writer.flush_block();
        }

        written += 1;
        line_count += 1;
    }

    writer.finish();
    let elapsed = start.elapsed();
    println!(
        "Converted {} records from '{}' to '{}' in {:.3}s ({:.0} records/s)",
        written,
        args.input,
        args.output,
        elapsed.as_secs_f64(),
        written as f64 / elapsed.as_secs_f64()
    );
}
