extern crate termsize;

use clap::Parser;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, ErrorKind::BrokenPipe, Result, Seek, SeekFrom, Write};
use std::process::exit;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "A cool program")]
struct Args {
    filename: String,

    #[arg(short='r', long, default_value_t=25)]
    header_repeat: u16,

    #[arg(short='m', long, default_value_t=25)]
    max_value_length: u16,

    #[arg(short, long)]
    line_numbers: bool,

}

#[derive(Debug)]
enum ColumnType {
    Textual,
    Numeric,
}

use ColumnType::*;

#[derive(Debug)]
struct Column {
    name: String,
    kind: ColumnType,
    max_length: usize,
}

impl Column {
    fn new(name: &str) -> Self {
        Column {
            name: String::from(name.trim()),
            kind: Numeric,
            max_length: name.len(),
        }
    }

    fn update(&mut self, value: &str) {
        self.max_length = std::cmp::max(self.max_length, value.len());
        self.kind = match self.kind {
            Textual => Textual,
            Numeric => match value.parse::<f64>() {
                Ok(_) => Numeric,
                Err(_) => Textual,
            },
        }
    }
}

struct DelimitedFile<T: BufRead> {
    reader: T,
    cols: Vec<Column>,
    header_bytes: usize,
    header_repeat: Option<u16>,
    max_value_length: Option <u16>,
}

impl<T> DelimitedFile<T>
    where T: BufRead + Seek,
{
    pub fn new(reader: T) -> Self {
        Self {
            reader: reader,
            cols: Vec::new(),
            header_bytes: 0,
            header_repeat: None,
            max_value_length: None,
        }
    }

    fn set_header_repeat(&mut self, header_repeat: Option<u16>) {
        self.header_repeat = header_repeat;
    }

    fn set_max_value_length(&mut self, max_value_length: Option<u16>) {
        self.max_value_length = max_value_length;
    }

    fn line_parse(l: &str) -> Vec<String> {
        l.split('\t')
            .map(|s| String::from(s.trim()))
            .collect::<Vec<String>>()
    }

    fn seek_to_data(&mut self) -> Result<()> {
        let _ = self.reader.seek(SeekFrom::Start(self.header_bytes as u64))?;
        Ok(())
    }

    // Read the header line of the file,
    // return the position after we finish
    // and the Vec of Columns
    fn read_headers(&mut self) -> Result<()> {
        let mut cols: Vec<Column> = Vec::new();
        let mut headers = String::new();
        self.header_bytes = self.reader.read_line(&mut headers)?;
        let headers = Self::line_parse(&headers);
        cols.extend(headers.iter().map(|h| Column::new(h)));
        self.cols = cols;
        Ok(())
    }

    // Read the file, noting size and type of all the data
    fn analyze_rows(&mut self) -> Result<()> {
        let mut line_str = String::new();
        while let Ok(bytes) = self.reader.read_line(&mut line_str) {
            if bytes == 0 {
                break;
            }
            let line = Self::line_parse(&line_str);
            for (i, value) in line.iter().enumerate() {
                if let Some(col) = self.cols.get_mut(i) {
                    col.update(value);
                }
            }
            line_str = String::new();
        }
        Ok(())
    }

    fn print_aligned_header(&mut self) -> Result<()> {
        let mut stdout = stdout().lock();
        for col in &self.cols {
            write!(stdout, "{}┼", "─".repeat(col.max_length))?;
        }
        writeln!(stdout)?;
        for col in &self.cols {
            write!(stdout, "{:-width$}│", col.name, width = col.max_length)?;
        }
        writeln!(stdout)?;
        for col in &self.cols {
            write!(stdout, "{}┼", "─".repeat(col.max_length))?;
        }
        writeln!(stdout)?;
        Ok(())
    }

    fn print_aligned_rows(&mut self) -> Result<()> {
        let mut stdout = stdout().lock();
        let mut line_str = String::new();
        let mut line_num = 0u16;

        while let Ok(bytes) = self.reader.read_line(&mut line_str) {
            if bytes == 0 {
                break;
            }
            line_num += 1;
            match self.header_repeat {
                Some(hr) => if line_num % hr == hr {
                    println!("here");
                    self.print_aligned_header()?;
                    ()
                },
                None => ()
            }

            let line = Self::line_parse(&line_str);
            for (i, value) in line.iter().enumerate() {
                let col = self.cols.get(i).unwrap();
                match col.kind {
                    Textual => write!(stdout, "{:-width$}│", value, width = col.max_length)?,
                    Numeric => write!(stdout, "{:>-width$}│", value, width = col.max_length)?,
                }
            }
            writeln!(stdout)?;
            line_str.truncate(0);
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("{:?}", args);
    //let rows = match termsize::get() {
    //    Some(size) => Some(size.rows),
    //    None => Some(args.header_repeat),
    //};

    let reader = match File::open(args.filename).map(BufReader::new) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Could no open file: {}", err);
            exit(1);
        }
    };

    let mut dfile = DelimitedFile::new(reader);
    dfile.set_header_repeat(Some(args.header_repeat));
    dfile.set_max_value_length(Some(args.max_value_length));
    dfile.read_headers()?;
    dfile.analyze_rows()?;
    dfile.seek_to_data()?;
    dfile.print_aligned_header()?;
    dfile.print_aligned_rows()?;

    match dfile.print_aligned_rows() {
        Ok(()) => (),
        Err(err) => match err.kind() {
            BrokenPipe => exit(0),
            _ => {
                eprintln!("Failed writing output: {}", err.kind());
                exit(1);
            }
        },
    };

    Ok(())
}
