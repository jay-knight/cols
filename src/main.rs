use std::io::{BufReader, BufRead, Seek, SeekFrom};
use std::fs::File;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    filename: String,
}

#[derive(Debug)]
enum ColType {
    Textual,
    Numeric,
}

#[derive(Debug)]
struct Col {
    name: String,
    kind: ColType,
    max_length: usize,
}

impl Col {
    fn new(name: &str) -> Self {
        Col {
            name: String::from(name.trim()),
            kind: ColType::Numeric,
            max_length: name.len(),
        }
    }

    fn update(&mut self, value: &str) {
        self.max_length = std::cmp::max(self.max_length, value.len());
        self.kind = match self.kind {
            ColType::Textual => ColType::Textual,
            ColType::Numeric => {
                match value.parse::<f64>() {
                    Ok(_) => ColType::Numeric,
                    Err(_) => ColType::Textual,
                }
            }
        }
    }
}

fn line_parse(l: &str) -> Vec<String> {
    l.split("\t").map(|s| String::from(s.trim())).collect::<Vec<String>>()
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let file = File::open(args.filename)?;
    let mut reader = BufReader::new(file);
    //let mut lines = reader.by_ref().lines();

    let mut cols: Vec<Col> = Vec::new();

    // Read the first line, set up your cols
    let mut headers = String::new();
    let _header_bytes = reader.read_line(&mut headers)?;
    let headers = line_parse(&headers);
    cols.extend(headers.iter().map(|h| Col::new(h)));

    // Read the file, noting size and type of all the data
    let mut line_str = String::new();
    while let Ok(bytes) = reader.read_line(&mut line_str) {
        if bytes == 0 {
            break;
        }
        let line = line_parse(&line_str);
        for (i,value) in line.iter().enumerate() {
            if let Some(col) = cols.get_mut(i) {
                col.update(value);
            }
        }
        line_str = String::new();
    }

    //Seek back to the data portion and print it out nicely
    let _ = reader.seek(SeekFrom::Start (0u64))?;
    let mut line_str = String::new();
    while let Ok(bytes) = reader.read_line(&mut line_str) {
        if bytes == 0 {
            break;
        }
        let line = line_parse(&line_str);
        for (i,value) in line.iter().enumerate() {
            let col = cols.get(i).unwrap();
            match col.kind {
                ColType::Textual => print!("{:-width$} ", value, width=col.max_length),
                ColType::Numeric => print!("{:>-width$} ", value, width=col.max_length),
            }

        }
        println!("");
        line_str.truncate(0);
    }

    Ok(())
}
