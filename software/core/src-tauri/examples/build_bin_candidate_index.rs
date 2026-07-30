use std::collections::HashMap;
use std::fs;
use std::time::Instant;

fn main() -> anyhow::Result<()> {

    let json_path = "../../../model/datasets/candidate/prefix_candidates.json";
    let output_path = "../../../model/datasets/candidate/prefix_candidates.bin";

    let start = Instant::now();

    let text = fs::read_to_string(json_path)?;
    let table: HashMap<String, Vec<i64>> = serde_json::from_str(&text)?;

    println!("Loaded {} prefixes from JSON in {:?}", table.len(), start.elapsed());

    let bytes = bincode::serialize(&table)?;

    fs::write(output_path, &bytes)?;

    println!("Wrote {} bytes to {}", bytes.len(), output_path);

    Ok(())
}