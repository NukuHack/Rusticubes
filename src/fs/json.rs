use std::{collections::HashMap, fmt, iter::Peekable, str::Chars};

#[derive(Debug, PartialEq, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

#[derive(Debug)]
pub enum JsonError {
    Parse {
        message: String,
        line: usize,
        column: usize,
    },
    Type {
        expected: &'static str,
        actual: Option<String>,
    },
    MissingField(&'static str),
    Custom(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JsonError::Parse { message, line, column } => 
                write!(f, "JSON parsing error at {}:{} - {}", line, column, message),
            JsonError::Type { expected, actual } => match actual {
                Some(actual) => write!(f, "Type mismatch, expected {}, got {}", expected, actual),
                None => write!(f, "Type mismatch, expected {}", expected),
            },
            JsonError::MissingField(field) => write!(f, "Missing required field: {}", field),
            JsonError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for JsonError {}

pub trait FromJson: Sized {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError>;
}

macro_rules! impl_from_json_for_num {
    ($($t:ty),*) => {
        $(impl FromJson for $t {
            fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
                match value {
                    JsonValue::Number(n) if n.fract() == 0.0 && *n >= 0.0 && *n <= Self::MAX as f64 => {
                        Ok(*n as Self)
                    }
                    _ => Err(JsonError::Type {
                        expected: stringify!($t),
                        actual: Some(format!("{:?}", value)),
                    }),
                }
            }
        })*
    };
}

impl_from_json_for_num!(u8, u16, u32, u64);

impl FromJson for bool {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Bool(b) => Ok(*b),
            _ => Err(JsonError::Type {
                expected: "bool",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

impl FromJson for f32 {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Number(n) => Ok(*n as f32),
            _ => Err(JsonError::Type {
                expected: "f32",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

impl FromJson for f64 {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Number(n) => Ok(*n),
            _ => Err(JsonError::Type {
                expected: "f64",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

impl FromJson for String {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            _ => Err(JsonError::Type {
                expected: "String",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Null => Ok(None),
            _ => Ok(Some(T::from_json(value)?)),
        }
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Array(arr) => arr.iter().map(T::from_json).collect(),
            _ => Err(JsonError::Type {
                expected: "Array",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

impl<V: FromJson> FromJson for HashMap<String, V> {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::Object(map) => map
                .iter()
                .map(|(k, v)| Ok((k.clone(), V::from_json(v)?)))
                .collect(),
            _ => Err(JsonError::Type {
                expected: "Object",
                actual: Some(format!("{:?}", value)),
            }),
        }
    }
}

pub struct JsonParser<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        JsonParser {
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    pub fn parse(input: &'a str) -> Result<JsonValue, JsonError> {
        Self::new(input).parse_self()
    }

    pub fn parse_self(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('t') | Some('f') | Some('n') => self.parse_small(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c.is_ascii_digit() || *c == '-' => self.parse_number(),
            _ => Err(self.error("Unexpected token")),
        }
    }

    fn parse_small(&mut self) -> Result<JsonValue, JsonError> {
        if self.starts_with("true") {
            self.expect("true")?;
            Ok(JsonValue::Bool(true))
        } else if self.starts_with("false") {
            self.expect("false")?;
            Ok(JsonValue::Bool(false))
        } else if self.starts_with("null") {
            self.expect("null")?;
            Ok(JsonValue::Null)
        } else {
            Err(self.error("Expected boolean or null"))
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let mut num_str = String::new();
        if self.chars.next_if_eq(&'-').is_some() {
            num_str.push('-');
        }

        while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit()) {
            num_str.push(c);
        }

        if self.chars.next_if_eq(&'.').is_some() {
            num_str.push('.');
            while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit()) {
                num_str.push(c);
            }
        }

        if self.chars.next_if(|c| matches!(c, 'e' | 'E')).is_some() {
            num_str.push('e');
            if let Some(c) = self.chars.next_if(|c| matches!(c, '+' | '-')) {
                num_str.push(c);
            }
            while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit()) {
                num_str.push(c);
            }
        }

        let num = num_str
            .parse::<f64>()
            .map_err(|_| self.error("Invalid number format"))?;

        if num.is_infinite() {
            return Err(self.error("Number is too large"));
        }

        Ok(JsonValue::Number(num))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        let mut result = String::new();
        self.expect_char('"')?;

        while let Some(c) = self.chars.next() {
            match c {
                '"' => return Ok(result),
                '\\' => result.push(self.parse_escape_sequence()?),
                '\n' => {
                    self.line += 1;
                    self.column = 0;
                    result.push(c);
                }
                _ => result.push(c),
            }
            self.column += 1;
        }

        Err(self.error("Unterminated string"))
    }

    fn parse_escape_sequence(&mut self) -> Result<char, JsonError> {
        Ok(match self
            .chars
            .next()
            .ok_or_else(|| self.error("Incomplete escape sequence"))?
        {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => self.parse_unicode_escape()?,
            c => return Err(self.error(&format!("Invalid escape sequence \\{}", c))),
        })
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonError> {
        let hex = (0..4)
            .map(|_| {
                self.chars
                    .next()
                    .and_then(|c| c.to_digit(16))
                    .ok_or_else(|| self.error("Invalid Unicode escape"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        std::char::from_u32(hex.iter().fold(0, |acc, &d| acc * 16 + d))
            .ok_or_else(|| self.error("Invalid Unicode code point"))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_char('[')?;
        let mut array = Vec::new();
        self.skip_whitespace();

        if self.chars.next_if_eq(&']').is_some() {
            return Ok(JsonValue::Array(array));
        }

        loop {
            array.push(self.parse_self()?);
            self.skip_whitespace();

            match self.chars.next() {
                Some(',') => {
                    self.skip_whitespace();
                    continue;
                }
                Some(']') => break,
                _ => return Err(self.error("Expected ',' or ']' in array")),
            }
        }

        Ok(JsonValue::Array(array))
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_char('{')?;
        let mut object = HashMap::new();
        self.skip_whitespace();

        if self.chars.next_if_eq(&'}').is_some() {
            return Ok(JsonValue::Object(object));
        }

        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            object.insert(key, self.parse_self()?);
            self.skip_whitespace();

            match self.chars.next() {
                Some(',') => {
                    self.skip_whitespace();
                    continue;
                }
                Some('}') => break,
                _ => return Err(self.error("Expected ',' or '}' in object")),
            }
        }

        Ok(JsonValue::Object(object))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.chars.next_if(|c| c.is_whitespace()) {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn expect(&mut self, s: &str) -> Result<(), JsonError> {
        for c in s.chars() {
            if self.chars.next_if_eq(&c).is_none() {
                return Err(self.error(&format!("Expected '{}'", c)));
            }
            self.column += 1;
        }
        Ok(())
    }

    fn expect_char(&mut self, c: char) -> Result<(), JsonError> {
        self.chars
            .next_if_eq(&c)
            .map(|_| self.column += 1)
            .ok_or_else(|| self.error(&format!("Expected '{}'", c)))
    }

    fn starts_with(&mut self, s: &str) -> bool {
        self.chars.clone().take(s.len()).eq(s.chars())
    }

    fn error(&self, msg: &str) -> JsonError {
        JsonError::Parse {
            message: msg.to_string(),
            line: self.line,
            column: self.column,
        }
    }
}

pub fn read_json_file(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok().filter(|s| !s.trim().is_empty())
}
