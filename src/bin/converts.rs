use clap::Parser;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use apache_avro::schema::{Name, RecordField, RecordFieldOrder, RecordSchema, UnionSchema};
use apache_avro::Schema;

/// Command line arguments for the converter.
#[derive(Parser, Debug)]
struct Args {
    /// Input file containing the "pairs" definition (e.g. a *.struct.line file).
    #[arg(short, long)]
    input: String,
    /// Output file where the Avro JSON schema will be written.
    #[arg(short, long)]
    output: String,
    /// Name of the Avro record to generate.
    #[arg(short = 'n', long)]
    name: String,
}

/// Map a single‑character type designator to its Avro primitive `Schema`.
fn avro_schema_from_char(c: char) -> Option<Schema> {
    match c.to_ascii_lowercase() {
        's' => Some(Schema::String),
        'l' => Some(Schema::Long),
        'd' => Some(Schema::Double),
        _ => None,
    }
}

fn main() {
    // Parse command line arguments.
    let args = Args::parse();

    // Open the input file and locate the first line that looks like a schema definition.
    let f = File::open(&args.input)
        .unwrap_or_else(|e| panic!("Failed to open input file '{}': {}", args.input, e));
    let mut reader = BufReader::new(f);
    let mut first_line = String::new();
    loop {
        let bytes = reader
            .read_line(&mut first_line)
            .expect("Failed to read line from input file");
        if bytes == 0 {
            panic!("Input file '{}' does not contain a schema definition line", args.input);
        }
        if first_line.trim().is_empty() {
            continue;
        }
        if first_line.contains(':') {
            break;
        }
        first_line.clear();
    }
    let schema_line = first_line.trim();

    // Build `RecordField` objects using `apache_avro` types.
    let mut fields_vec = Vec::new();
    for (idx, pair) in schema_line.split(',').enumerate() {
        if pair.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.split(':').collect();
        if parts.len() != 2 {
            panic!("Invalid field definition '{}', expected 'name:type'", pair);
        }
        let field_name = parts[0].trim();
        let type_token = parts[1].trim();
        let type_char = type_token
            .chars()
            .next()
            .expect("Empty type token in schema definition");
        let primitive_schema =
            avro_schema_from_char(type_char).unwrap_or_else(|| panic!("Unsupported type character '{}' in field '{}'. Supported: s, l, d (case‑insensitive).", type_char, field_name));
        // Determine nullability: lower‑case => nullable (union with null), upper‑case => non‑null.
        let field_schema = if type_char.is_ascii_uppercase() {
            primitive_schema
        } else {
            let union = UnionSchema::new(vec![Schema::Null, primitive_schema])
                .expect("Failed to create nullable union schema");
            Schema::Union(union)
        };

        let record_field = RecordField::builder()
            .name(field_name.to_string())
            .schema(field_schema)
            .order(RecordFieldOrder::Ascending)
            .position(idx)
            .build();
        fields_vec.push(record_field);
    }

    // Build the lookup map required by the Record schema.
    let lookup: BTreeMap<String, usize> = fields_vec
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i))
        .collect();

    // Build the full Avro Record schema using the name supplied via CLI.
    let record_schema = RecordSchema::builder()
        .name(Name::new(&args.name).expect("Invalid record name"))
        .fields(fields_vec)
        .lookup(lookup)
        .build();

    // Serialize the schema to pretty JSON using the `Serialize` impl provided by apache_avro.
    let json_schema = serde_json::to_string_pretty(&Schema::Record(record_schema))
        .expect("Failed to serialize Avro schema to JSON");

    // Write the JSON schema to the output file.
    let mut out_file = File::create(&args.output)
        .unwrap_or_else(|e| panic!("Failed to create output file '{}': {}", args.output, e));
    out_file
        .write_all(json_schema.as_bytes())
        .expect("Failed to write Avro schema to output file");

    println!("Avro schema written to {}", args.output);
}
