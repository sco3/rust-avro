use apache_avro::{AvroResult, Schema};
use apache_avro::schema::UnionSchema;



fn main( ) {
    let fieldVariants = vec!(Schema::Null,Schema::Double);
    match  UnionSchema::new(fieldVariants) {
        Ok(s) => {println !("{s:?}")}
        Err(e) => {println !("{e:?}")}
    }
}