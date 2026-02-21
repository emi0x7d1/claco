use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct Frame {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
}

#[derive(Serialize, Deserialize)]
struct Cell {
    ch: String,
}

fn main() {
    let content = fs::read_to_string("frame1.json").unwrap();
    let frame: Frame = serde_json::from_str(&content).unwrap();
    for (i, cell) in frame.cells.iter().enumerate() {
        if cell.ch == "❯" {
            let y = i / frame.cols;
            let x = i % frame.cols;
            println!("Found ❯ at y={}, x={}", y, x);
        }
    }
}
