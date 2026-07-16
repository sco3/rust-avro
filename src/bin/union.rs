use apache_avro::Schema;
use apache_avro::schema::UnionSchema;

fn main() {
    let field_variants = vec![Schema::Null, Schema::Double];
    match UnionSchema::new(field_variants) {
        Ok(s) => {
            println!("{s:?}");
        }
        Err(e) => {
            println!("{e:?}");
        }
    }
}
