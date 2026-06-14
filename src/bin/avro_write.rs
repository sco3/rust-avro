use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use apache_avro::schema::RecordField;
use apache_avro::{Schema, Writer};
use apache_avro::types::Value;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "avro_write", about = "Convert pipe-separated CSV to Avro format")]
struct Args {
    /// Input pipe-separated CSV file
    #[arg(short, long)]
    input: String,

    /// Output Avro data file
    #[arg(short, long)]
    output: String,

    /// Avro schema file (.avsc)
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

fn parse_field_value(raw: &str, field_type: FieldType, union_index: u32) -> Value {
    if raw == "\\N" {
        return Value::Union(0, Box::new(Value::Null));
    }
    let inner = match field_type {
        FieldType::String => Value::String(raw.to_string()),
        FieldType::Long => Value::Long(raw.parse::<i64>().unwrap_or_else(|e| {
            panic!("Failed to parse '{}' as long: {}", raw, e);
        })),
        FieldType::Double => Value::Double(raw.parse::<f64>().unwrap_or_else(|e| {
            panic!("Failed to parse '{}' as double: {}", raw, e);
        })),
        FieldType::Int => Value::Int(raw.parse::<i32>().unwrap_or_else(|e| {
            panic!("Failed to parse '{}' as int: {}", raw, e);
        })),
        FieldType::Boolean => Value::Boolean(raw.parse::<bool>().unwrap_or_else(|e| {
            panic!("Failed to parse '{}' as boolean: {}", raw, e);
        })),
        FieldType::Float => Value::Float(raw.parse::<f32>().unwrap_or_else(|e| {
            panic!("Failed to parse '{}' as float: {}", raw, e);
        })),
    };
    Value::Union(union_index, Box::new(inner))
}

fn extract_field_info(field: &RecordField) -> (FieldType, u32) {
    match &field.schema {
        Schema::Union(u) => {
            let variants = u.variants();
            if variants.len() == 2 && matches!(variants[0], Schema::Null) {
                    let t = match &variants[1] {
                        Schema::String => FieldType::String,
                        Schema::Long => FieldType::Long,
                        Schema::Double => FieldType::Double,
                        Schema::Int => FieldType::Int,
                        Schema::Boolean => FieldType::Boolean,
                        Schema::Float => FieldType::Float,
                        _ => FieldType::String,
                    };
                    return (t, 1u32);
                }
            (FieldType::String, 0u32)
        }
        Schema::String => (FieldType::String, 0u32),
        Schema::Long => (FieldType::Long, 0u32),
        Schema::Double => (FieldType::Double, 0u32),
        Schema::Int => (FieldType::Int, 0u32),
        Schema::Boolean => (FieldType::Boolean, 0u32),
        Schema::Float => (FieldType::Float, 0u32),
        _ => (FieldType::String, 0u32),
    }
}

fn main() {
    let args = Args::parse();

    // Read and parse the schema
    let schema_str = std::fs::read_to_string(&args.schema)
        .unwrap_or_else(|e| panic!("Failed to read schema file '{}': {}", args.schema, e));
    let schema = Schema::parse_str(&schema_str)
        .unwrap_or_else(|e| panic!("Failed to parse schema: {}", e));

    let record_schema = match &schema {
        Schema::Record(rs) => rs,
        _ => panic!("Schema must be a record type"),
    };

    let field_infos: Vec<(FieldType, u32)> = record_schema.fields.iter().map(extract_field_info).collect();
    let num_fields = field_infos.len();

    // Open input CSV
    let csv_file = File::open(&args.input)
        .unwrap_or_else(|e| panic!("Failed to open input file '{}': {}", args.input, e));
    let reader = BufReader::new(csv_file);

    // Open output Avro file
    let out_file = File::create(&args.output)
        .unwrap_or_else(|e| panic!("Failed to create output file '{}': {}", args.output, e));
    let mut writer = Writer::new(&schema, out_file);

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

        let mut record = apache_avro::types::Record::new(&schema)
            .expect("Failed to create record from schema");

        for i in 0..num_fields {
            let (field_type, union_idx) = field_infos[i];
            let value = parse_field_value(columns[i], field_type, union_idx);
            record.fields[i].1 = value;
        }

        writer
            .append(record)
            .expect("Failed to append record to writer");
        written += 1;
        line_count += 1;
    }

    writer.flush().expect("Failed to flush writer");
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
