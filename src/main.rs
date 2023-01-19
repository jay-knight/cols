use std::io::{
    stdout,
    BufReader,
    BufRead,
    Write,
    Seek,
    SeekFrom,
    Result,
    ErrorKind::BrokenPipe,
};
use std::fs::File;
use std::process::exit;
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

use ColType::*;

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
            kind: Numeric,
            max_length: name.len(),
        }
    }

    fn update(&mut self, value: &str) {
        self.max_length = std::cmp::max(self.max_length, value.len());
        self.kind = match self.kind {
            Textual => Textual,
            Numeric => {
                match value.parse::<f64>() {
                    Ok(_) => Numeric,
                    Err(_) => Textual,
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
fn read_headers<T: BufRead>(reader: &mut T) -> Result<(Vec<Col>, u64)> {
    let mut cols: Vec<Col> = Vec::new();
    let mut headers = String::new();
    let header_bytes = reader.read_line(&mut headers)?;
    let headers = line_parse(&headers);
    cols.extend(headers.iter().map(|h| Col::new(h)));
    Ok((cols, header_bytes as u64))
}

// Read the file, noting size and type of all the data
fn analyze_rows<T: BufRead>(reader: &mut T, cols: &mut Vec<Col>) -> Result<()> {
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

fn print_aligned_header(cols: &Vec<Col>) -> Result<()>{
    let mut stdout = stdout().lock();
    for col in cols {
        write!(stdout, "{:^-width$} ", col.name, width=col.max_length)?;
    }
    writeln!(stdout, "")?;
    Ok(())
}

fn print_aligned_rows<T: BufRead>(reader: &mut T, cols: &Vec<Col>) -> Result<()> {
    let mut stdout = stdout().lock();
    let mut line_str = String::new();
    while let Ok(bytes) = reader.read_line(&mut line_str) {
        if bytes == 0 {
            break;
        }
        let line = line_parse(&line_str);
        for (i,value) in line.iter().enumerate() {
            let col = cols.get(i).unwrap();
            match col.kind {
                Textual => write!(stdout, "{:-width$} ", value, width=col.max_length)?,
                Numeric => write!(stdout, "{:>-width$} ", value, width=col.max_length)?,
            }

        }
        writeln!(stdout, "")?;
        line_str.truncate(0);
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let file = match File::open(args.filename) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Could no open file: {}", err);
            exit(1);
        }
    };
    let mut reader = BufReader::new(file);

    // Read the first line, set up your cols
    let (mut cols, header_bytes) = read_headers(&mut reader)?;

    let _position = analyze_rows(&mut reader, &mut cols);

    print_aligned_header(&cols)?;

    //Seek back to the data portion and print it out nicely
    reader.seek(SeekFrom::Start (header_bytes as u64))?;
    match print_aligned_rows(&mut reader, &cols) {
        Ok(()) => (),
        Err(err) => {
            match err.kind() {
                BrokenPipe => exit(0),
                _ => {
                    eprintln!("Failed writing output: {}", err.kind());
                    exit(1);
                }
            }
        }
    };

    Ok(())
}
