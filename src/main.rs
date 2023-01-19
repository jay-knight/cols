use std::io::{BufReader, BufRead, Seek, SeekFrom, Result};
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

// Read the header line of the file,
// return the position after we finish
// and the Vec of Cols
fn read_headers(reader: &mut BufReader<File>) -> Result<(Vec<Col>, u64)> {
    let mut cols: Vec<Col> = Vec::new();
    let mut headers = String::new();
    let header_bytes = reader.read_line(&mut headers)?;
    let headers = line_parse(&headers);
    cols.extend(headers.iter().map(|h| Col::new(h)));
    Ok((cols, header_bytes as u64))
}

// Read the file, noting size and type of all the data
fn analyze_rows(reader: &mut BufReader<File>, cols: &mut Vec<Col>) -> Result<()> {
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
    Ok(())
}

fn print_aligned_header(cols: &Vec<Col>) {
    for col in cols {
        print!("{:^-width$} ", col.name, width=col.max_length)
    }
    println!("");
}

fn print_aligned_rows(reader: &mut BufReader<File>, cols: &Vec<Col>) -> Result<()> {
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

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let file = File::open(args.filename)?;
    let mut reader = BufReader::new(file);

    // Read the first line, set up your cols
    let (mut cols, header_bytes) = read_headers(&mut reader)?;

    let _position = analyze_rows(&mut reader, &mut cols);

    print_aligned_header(&cols);

    //Seek back to the data portion and print it out nicely
    reader.seek(SeekFrom::Start (header_bytes as u64))?;
    print_aligned_rows(&mut reader, &cols)?;

    Ok(())
}
