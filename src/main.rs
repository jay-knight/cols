extern crate termsize;

use clap::Parser;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, ErrorKind::BrokenPipe, Result, Seek, SeekFrom, Write};
use std::process::exit;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "A cool program")]
struct Args {
    filename: String,

    #[arg(short = 'r', long, default_value_t = 25)]
    header_repeat: u16,

    #[arg(short = 'R', long)]
    no_header_repeat: bool,

    #[arg(short = 't', long)]
    truncate_values: Option<u16>,

    #[arg(short = 'T', long)]
    no_truncate_long: bool,

    #[arg(short = 'l', long)]
    line_numbers: bool,

    #[arg(short = 'b', long)]
    borders: bool,
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
    max_length: u16,
}

impl Column {
    fn new(name: &str) -> Self {
        Column {
            name: String::from(name.trim()),
            kind: Numeric,
            max_length: name.len() as u16,
        }
    }

    fn update(&mut self, value: &str) {
        self.max_length = std::cmp::max(self.max_length, value.len() as u16);
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
    max_value_length: Option<u16>,
    borders: bool,
}

impl<T> DelimitedFile<T>
where
    T: BufRead + Seek,
{
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            cols: Vec::new(),
            header_bytes: 0,
            header_repeat: None,
            max_value_length: None,
            borders: false,
        }
    }

    fn set_header_repeat(&mut self, header_repeat: Option<u16>) {
        self.header_repeat = header_repeat;
    }

    fn set_max_value_length(&mut self, max_value_length: Option<u16>) {
        self.max_value_length = max_value_length;
    }

    fn set_borders(&mut self, borders: bool) {
        self.borders = borders;
    }

    fn line_parse(l: &str) -> Vec<String> {
        l.split('\t')
            .map(|s| String::from(s.trim()))
            .collect::<Vec<String>>()
    }

    fn seek_to_data(&mut self) -> Result<()> {
        let _ = self
            .reader
            .seek(SeekFrom::Start(self.header_bytes as u64))?;
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
        if self.borders {
            for col in &self.cols {
                write!(stdout, "{}┼", "─".repeat(self.print_length(col)))?;
            }
            writeln!(stdout)?;
        }
        for col in &self.cols {
            write!(
                stdout,
                "{:-width$}",
                self.format_value(&col.name, col),
                width = self.print_length(col)
            )?;
        }
        if self.borders {
            writeln!(stdout)?;
            for col in &self.cols {
                write!(stdout, "{}┼", "─".repeat(self.print_length(col)))?;
            }
            writeln!(stdout)?;
        }
        Ok(())
    }

    fn print_length(&self, col: &Column) -> usize {
        match self.max_value_length {
            Some(length) => std::cmp::min(col.max_length, length) as usize,
            None => col.max_length as usize,
        }
    }

    fn format_value(&self, value: &str, col: &Column) -> String {
        let print_length = self.print_length(col);
        let truncated: &str = if print_length < value.len() {
            &value[0..print_length]
        } else {
            value
        };
        let sep = if self.borders { "│" } else { " " };
        match col.kind {
            Textual => format!("{truncated:-print_length$}{sep}"),
            Numeric => format!("{truncated:>-print_length$}{sep}"),
        }
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
            if let Some(hr) = self.header_repeat {
                if line_num % hr == 0 {
                    self.print_aligned_header()?;
                }
            }

            let line = Self::line_parse(&line_str);
            for (i, value) in line.iter().enumerate() {
                let col = self.cols.get(i).expect("No column for i=");
                write!(stdout, "{}", self.format_value(value, col))?;
            }
            writeln!(stdout)?;
            line_str.truncate(0);
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    //let rows = match termsize::get() {
    //    Some(size) => Some(size.rows),
    //    None => Some(args.header_repeat),
    //};

    let reader = match File::open(args.filename).map(BufReader::new) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Could no open file: {err}");
            exit(1);
        }
    };

    let mut dfile = DelimitedFile::new(reader);
    dfile.set_header_repeat(if args.no_header_repeat {
        None
    } else {
        Some(args.header_repeat)
    });
    dfile.set_max_value_length(if args.no_truncate_long {
        None
    } else {
        args.truncate_values
    });
    dfile.set_borders(args.borders);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column() {
        let mut text_column = Column {
            name: "Test Col".to_string(),
            kind: Textual,
            max_length: 0,
        };
        text_column.update("1234");
        assert_eq!(text_column.max_length, 4);
        text_column.update("12345678");
        assert_eq!(text_column.max_length, 8);
    }

    #[test]
    fn test_print_length() {
        use std::io::Cursor;

        let buff = Cursor::new("header1\theader2\nvalue1\tvalue2\n");
        let mut dfile = DelimitedFile::new(buff);
        dfile.set_max_value_length(Some(4));
        dfile.read_headers().unwrap();
        assert_eq!(dfile.cols.len(), 2);
        dfile.analyze_rows().unwrap();
        assert_eq!(dfile.print_length(&dfile.cols[0]), 4);
        dfile.set_max_value_length(Some(40));
        assert_eq!(dfile.print_length(&dfile.cols[0]), 7);
    }
}
