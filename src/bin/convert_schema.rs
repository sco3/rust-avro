use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};
use avro_rs::Schema;
use avro_rs::schema:: { RecordField};
#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    input: String,
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();
    let input = args.input;
    let f = File::open(&input).expect(&format!("File not found {input}"));
    let reader = BufReader::new(f);
    reader.lines().for_each(|l| {
        if let Ok(line) = l {
            line.split(",").for_each(|field| {
                let parts:Vec<&str> = field.split (":").collect();
                println! ("{:?} -> {:?}",parts.get(0), parts.get(1));
            })
        }
    })
}
