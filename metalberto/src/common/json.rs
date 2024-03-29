use core::{fmt, result::Result};
use std::collections::HashMap;
use std::str;

//
//  Value stuff
//
#[derive(Debug)]
#[allow(dead_code)]
pub enum Value {
    Array { value: Vec<Value> },
    Boolean { value: bool },
    Dict { value: HashMap<String, Value> },
    Float { value: f64 },
    Integer { value: i64 },
    Null,
    String { value: String },
}

//
//  Error stuff
//
#[derive(Debug)]
#[allow(dead_code)] // we allow dead code since we don't read the error codes ourselves, that is for the user
pub enum ErrorType {
    ExpectedArrayCloseOrComma,
    ExpectedDictKey,
    ExpectedDictColonAfterKey { key: String },
    ExpectedDictCloseOrComma,
    UnexpectedEndOfFile,
    UnknownKeyword { keyword: String }
}

pub struct Error {
    pub line: usize,
    pub error: ErrorType,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} on line {}", self.error, self.line)
    }
}

//
//  Parser stuff
//
#[allow(dead_code)]
struct Parser<'a> {
    line: usize,
    caret: usize,
    document: &'a [u8],
}

#[allow(dead_code)]
impl Parser<'_> {
    // constructor
    fn new<'a>(document: &'a String) -> Parser<'a> {
        Parser {
            line: 0,
            caret: 0,
            document: document.as_bytes(),
        }
    }

    // entry function
    fn parse(&mut self) -> Result<Value, Error> {
        self.skip_whitespace()?;
        
        // println!("entering parse, seeing: '{}'", char::from(self.peek()?));
        match self.peek()? {
            b'{' => self.dict(),
            b'[' => self.array(),
            b'"' => Ok(Value::String { value: self.string()? }),
            b't' => { self.word(b"true")?; Ok(Value::Boolean { value: true }) },
            b'f' => { self.word(b"false")?; Ok(Value::Boolean { value: false }) },
            b'n' => { self.word(b"null")?; Ok(Value::Null) },
            _ => self.number()
        }
    }

    // specific parsers
    fn array(&mut self) -> Result<Value, Error> {
        self.advance()?;
        let mut array: Vec<Value> = Vec::new();

        loop {
            self.skip_whitespace()?;

            if self.check(b']')? {
                return Ok(Value::Array { value: array });
            }

            array.push(self.parse()?);

            self.skip_whitespace()?;
            if self.peek()? != b']' && !self.check(b',')? {
                return Err(self.error(ErrorType::ExpectedArrayCloseOrComma));
            }
        }
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.caret;
        self.check(b'-')?;

        if self.peek()? == b'0' {
            self.advance()?;
        } else {
            while self.peek()?.is_ascii_digit() {
                self.advance()?;
            }
        }

        if !self.check(b'.')? { // no dot so integer
            let as_str = str::from_utf8(&self.document[start..self.caret]).unwrap();
            return Ok(Value::Integer { value: as_str.parse().unwrap() });
        }

        while self.peek()?.is_ascii_digit() {
            self.advance()?;
        }
        
        if self.check(b'e')? || self.check(b'E')? {
            while self.peek()?.is_ascii_digit() {
                self.advance()?;
            }
        }

        let as_str = str::from_utf8(&self.document[start..self.caret]).unwrap();
        Ok(Value::Float { value: as_str.parse().unwrap() })
    }

    fn dict(&mut self) -> Result<Value, Error> {
        self.advance()?; // skip '{'
        let mut dict: HashMap<String, Value> = HashMap::new();

        loop {
            self.skip_whitespace()?;

            if self.check(b'}')? {
                return Ok(Value::Dict { value: dict });
            }

            if !self.peek()? == b'"' {
                return Err(self.error(ErrorType::ExpectedDictKey));
            }
            
            let key = self.string()?;
            
            self.skip_whitespace()?;

            if !self.check(b':')? {
                return Err(self.error(ErrorType::ExpectedDictColonAfterKey { key: key }));
            }

            self.skip_whitespace()?;
            dict.insert(key, self.parse()?);
            self.skip_whitespace()?;

            if self.peek()? != b'}' && !self.check(b',')? {
                return Err(self.error(ErrorType::ExpectedDictCloseOrComma));
            }
        }
    }

    fn string(&mut self) -> Result<String, Error> {
        self.advance()?;
        let start = self.caret;
        
        loop {
            if self.check(b'"')? {
                // this can probably be from_utf8_unchecked but what do I know, lets leave unsafe for what it
                // is for now
                let string = String::from_utf8(self.document[start..(self.caret - 1)].to_vec()).unwrap();
                // println!("Parsed string: '{}'", string);
                return Ok(string);
            }
            self.check(b'\\')?;
            self.caret += 1;
        }
    }

    fn word(&mut self, characters: &[u8]) -> Result<(), Error> {
        for char in characters.iter() {
            if self.advance()? != *char {
                return Err(self.error(ErrorType::UnknownKeyword { keyword: String::from_utf8(characters.to_vec()).unwrap() }));
            }
        }
        
        // println!("parsed keyword!");
        Ok(())
    }

    // utility
    fn advance(&mut self) -> Result<u8, Error> {
        let ch = self.peek()?;
        self.caret += 1;
        Ok(ch)
    }

    fn peek(&self) -> Result<u8, Error> {
        if self.caret >= self.document.len() {
            return Err(self.error(ErrorType::UnexpectedEndOfFile));
        }
        // println!(" peek: i: {:03}, {}", self.caret, char::from(self.document[self.caret]));
        Ok(self.document[self.caret])
    }

    fn check(&mut self, expected: u8) -> Result<bool, Error> {
        // println!("check: i: {}, '{}' (?: '{}')", self.caret, char::from(self.document[self.caret]), char::from(expected));
        if self.peek()? != expected {
            return Ok(false);
        }

        self.advance()?;
        Ok(true)
    }

    fn error(&self, error: ErrorType) -> Error {
        Error {
            line: self.line,
            error: error,
        }
    }

    fn skip_whitespace(&mut self) -> Result<(), Error> {
        loop {
            match self.peek()? {
                b' ' => self.caret += 1,
                b'\t' => self.caret += 1,
                b'\n' => {
                    self.caret += 1;
                    self.line += 1
                }
                _ => break,
            }
        }
        Ok(())
    }
}

//
//  Public interface
//
#[allow(dead_code)]
pub fn parse_string<'a>(document: &'a String) -> Result<Value, Error> {
    let mut parser = Parser::new(document);
    parser.parse()
}