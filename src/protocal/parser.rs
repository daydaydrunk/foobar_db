use crate::protocal::resp::RespValue;
use bytes::{BufMut, BytesMut};
use std::borrow::Cow;

const MAX_ITERATIONS: usize = 128; // 设置最大循环次数
const CRLF_LEN: usize = 2;
const BUFFER_INIT_SIZE: usize = 1024;
const CR: u8 = b'\r';
const LF: u8 = b'\n';
const NEXT: usize = 1;
const NO_REMAINING: usize = 0;

type ParseResult = Result<Option<RespValue<'static>>, ParseError>;

#[derive(Debug, PartialEq, Clone)]
pub enum ParseError {
    InvalidFormat(Cow<'static, str>),
    InvalidLength,
    UnexpectedEof,
    Overflow,
    NotEnoughData,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParseState {
    Index {
        pos: usize,
    },
    ReadingLength {
        pos: usize,
        value: i64,
        negative: bool,
        type_char: u8,
    },
    ReadingBulkString {
        start_pos: usize,
        remaining: usize,
    },
    ReadingSimpleString {
        pos: usize,
    },
    ReadingError {
        pos: usize,
    },
    ReadingInteger {
        pos: usize,
    },
    // Nested structures whitch use stack to store and parse
    ReadingArray {
        pos: usize,     // 当前解析位置
        total: usize,   // 数组总长度
        current: usize, // 当前解析元素位置
        elements: Vec<RespValue<'static>>,
    },
    // Outcomes
    Error(ParseError),
    Complete(Option<RespValue<'static>>),
}

#[derive(Debug, Clone)]
pub struct Parser {
    pub buffer: BytesMut,
    state: ParseState,
    max_depth: usize,
    max_length: usize,
    nested_stack: Vec<ParseState>,
}

impl Parser {
    pub fn new(max_depth: usize, max_length: usize) -> Self {
        Parser {
            buffer: BytesMut::with_capacity(BUFFER_INIT_SIZE),
            state: ParseState::Index { pos: 0 },
            max_depth,
            max_length,
            nested_stack: Vec::with_capacity(max_depth),
        }
    }

    pub fn read_buf(&mut self, buf: &[u8]) {
        self.buffer.extend_from_slice(buf);
    }

    #[inline]
    fn find_crlf(&self, start: usize) -> Option<usize> {
        let mut pos = start;
        while pos < self.buffer.len().saturating_sub(1) {
            match (self.buffer.get(pos), self.buffer.get(pos + 1)) {
                (Some(&b'\r'), Some(&b'\n')) => return Some(pos),
                (Some(_), _) => pos += 1,
                _ => break,
            }
        }
        None
    }

    #[inline]
    fn handle_index(&mut self, index: usize) -> ParseState {
        if index >= self.buffer.len() {
            return ParseState::Index { pos: index };
        }

        match self.buffer[index] {
            // Simple Strings 以 "+" 开头，直接读取到 CRLF
            b'+' => ParseState::ReadingSimpleString { pos: index + 1 },

            // Errors 以 "-" 开头，直接读取到 CRLF
            b'-' => ParseState::ReadingError { pos: index + 1 },

            // Integers 以 ":" 开头，直接读取到 CRLF
            b':' => ParseState::ReadingInteger { pos: index + 1 },

            // Bulk Strings 以 "$" 开头，需要先读取长度
            b'$' => ParseState::ReadingLength {
                value: 0,
                negative: false,
                pos: index + 1,
                type_char: b'$',
            },

            // Arrays 以 "*" 开头，需要先读取长度
            b'*' => ParseState::ReadingLength {
                value: 0,
                negative: false,
                pos: index + 1,
                type_char: b'*',
            },

            // 其他字符都是非法的
            _ => ParseState::Error(ParseError::InvalidFormat("Invalid type marker".into())),
        }
    }

    #[inline]
    fn handle_length(
        &mut self,
        pos: usize,
        value: i64,
        negative: bool,
        type_char: u8,
    ) -> ParseState {
        return match self.buffer.get(pos) {
            Some(&b) => match b {
                b'0'..=b'9' => {
                    let new_value = match value.checked_mul(10).and_then(|v| {
                        if negative {
                            v.checked_sub((b - b'0') as i64)
                        } else {
                            v.checked_add((b - b'0') as i64)
                        }
                    }) {
                        Some(v) => v,
                        None => {
                            return ParseState::Error(ParseError::Overflow);
                        }
                    };

                    ParseState::ReadingLength {
                        pos: pos + 1,
                        value: new_value,
                        negative,
                        type_char,
                    }
                }
                b'-' => ParseState::ReadingLength {
                    pos: pos + 1,
                    value,
                    negative: true,
                    type_char,
                },
                b'\r' => match self.buffer.get(pos + 1) {
                    Some(&b'\n') => match type_char {
                        b'$' => {
                            if value == -1 {
                                ParseState::Complete(Some(RespValue::Null))
                            } else if value < 0 {
                                ParseState::Error(ParseError::InvalidLength)
                            } else {
                                ParseState::ReadingBulkString {
                                    start_pos: pos + 2,
                                    remaining: value as usize,
                                }
                            }
                        }
                        b'*' => {
                            if value == -1 {
                                ParseState::Complete(Some(RespValue::Array(None)))
                            } else if value < 0 {
                                ParseState::Error(ParseError::InvalidLength)
                            } else {
                                ParseState::ReadingArray {
                                    pos: pos + 2,
                                    total: value as usize,
                                    elements: Vec::with_capacity(value as usize),
                                    current: 0,
                                }
                            }
                        }
                        b':' => ParseState::Complete(Some(RespValue::Integer(value))),
                        _ => ParseState::Error(ParseError::InvalidFormat(
                            "Invalid length type".into(),
                        )),
                    },
                    _ => ParseState::Error(ParseError::InvalidFormat(
                        "Expected \\n after \\r".into(),
                    )),
                },
                _ => ParseState::Error(ParseError::InvalidFormat(
                    "Invalid character in length".into(),
                )),
            },
            None => ParseState::Error(ParseError::UnexpectedEof),
        };
    }

    #[inline]
    fn handle_bulk_string(&mut self, start_pos: usize, remaining: usize) -> ParseState {
        if remaining > self.max_length {
            return ParseState::Error(ParseError::InvalidLength);
        } else if remaining == NO_REMAINING {
            return ParseState::Complete(Some(RespValue::BulkString(None)));
        }

        let required_len = start_pos + remaining + CRLF_LEN; // +2 for CRLF
        if self.buffer.len() < required_len {
            return ParseState::Error(ParseError::NotEnoughData);
        }

        if self.buffer[start_pos + remaining] != CR
            || self.buffer[start_pos + remaining + NEXT] != LF
        {
            return ParseState::Error(ParseError::InvalidFormat("Missing CRLF".into()));
        }

        match String::from_utf8(self.buffer[start_pos..start_pos + remaining].to_vec()) {
            Ok(content) => ParseState::Complete(Some(RespValue::BulkString(Some(content.into())))),
            Err(_) => ParseState::Error(ParseError::InvalidFormat("Invalid UTF-8".into())),
        }
    }

    #[inline]
    fn handle_array(
        &mut self,
        pos: usize,
        total: usize,
        current: usize,
        elements: Vec<RespValue<'static>>,
    ) -> ParseState {
        if total == 0 {
            return ParseState::Complete(Some(RespValue::Array(Some(elements))));
        }
        if current >= total {
            return ParseState::Complete(Some(RespValue::Array(Some(elements))));
        }

        // Store current array state
        let arr = ParseState::ReadingArray {
            pos,
            total,
            elements,
            current,
        };
        self.nested_stack.push(arr);

        // Start parsing next element from current position
        ParseState::Index { pos }
    }

    #[inline]
    fn handle_simple_string(&mut self, pos: usize) -> ParseState {
        match self.find_crlf(pos) {
            Some(end_pos) => {
                let bytes = self.buffer[pos..end_pos].to_vec();
                let string = String::from_utf8_lossy(&bytes).into_owned().into();
                ParseState::Complete(Some(RespValue::SimpleString(string)))
            }
            None => ParseState::Error(ParseError::UnexpectedEof),
        }
    }

    #[inline]
    fn handle_error(&mut self, pos: usize) -> ParseState {
        match self.find_crlf(pos) {
            Some(end_pos) => {
                let bytes = self.buffer[pos..end_pos].to_vec();
                let string = String::from_utf8_lossy(&bytes).into_owned().into();
                ParseState::Complete(Some(RespValue::Error(string)))
            }
            None => ParseState::Error(ParseError::UnexpectedEof),
        }
    }

    #[inline]
    fn handle_integer(&mut self, pos: usize) -> ParseState {
        match self.find_crlf(pos) {
            Some(end_pos) => {
                let mut value = 0i64;
                let mut negative = false;
                let mut start = pos;

                match self.buffer.get(pos) {
                    Some(&b'-') => {
                        negative = true;
                        start = pos + 1;
                    }
                    _ => {}
                }

                for &b in &self.buffer[start..end_pos] {
                    match b {
                        b'0'..=b'9' => {
                            value = match value.checked_mul(10).and_then(|v| {
                                if negative {
                                    v.checked_sub((b - b'0') as i64)
                                } else {
                                    v.checked_add((b - b'0') as i64)
                                }
                            }) {
                                Some(v) => v,
                                None => {
                                    return ParseState::Error(ParseError::Overflow);
                                }
                            };
                        }
                        _ => {
                            return ParseState::Error(ParseError::InvalidFormat(
                                "Invalid integer format".into(),
                            ));
                        }
                    }
                }
                ParseState::Complete(Some(RespValue::Integer(value)))
            }
            None => ParseState::Error(ParseError::UnexpectedEof),
        }
    }

    pub fn try_parse(&mut self) -> ParseResult {
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(ParseError::InvalidFormat(
                    "Maximum parsing iterations exceeded".into(),
                ));
            }

            println!(
                "{:?} | state={:?} | buffer={:?}",
                iterations,
                self.state,
                String::from_utf8_lossy(&self.buffer)
            );

            let next_state = match &self.state {
                ParseState::Index { pos } => self.handle_index(*pos),
                ParseState::ReadingArray {
                    pos,
                    total,
                    elements,
                    current,
                } => self.handle_array(*pos, *total, *current, elements.clone()),
                ParseState::ReadingLength {
                    pos,
                    value,
                    negative,
                    type_char,
                } => self.handle_length(*pos, *value, *negative, *type_char),
                ParseState::ReadingBulkString {
                    start_pos,
                    remaining,
                } => self.handle_bulk_string(*start_pos, *remaining),
                ParseState::ReadingSimpleString { pos } => self.handle_simple_string(*pos),
                ParseState::ReadingError { pos } => self.handle_error(*pos),
                ParseState::ReadingInteger { pos } => self.handle_integer(*pos),
                ParseState::Error(error) => ParseState::Error(error.clone()),
                ParseState::Complete(value) => ParseState::Complete(value.clone()),
            };

            match next_state {
                ParseState::Complete(Some(value)) => {
                    if let Some(ParseState::ReadingArray {
                        total,
                        elements,
                        pos,
                        current,
                    }) = self.nested_stack.last_mut()
                    {
                        elements.push(value.clone());
                        if *current + 1 < *total {
                            *current += 1;
                            continue;
                        } else {
                            let completed_array = RespValue::Array(Some(elements.clone()));
                            self.nested_stack.pop();
                            if let Some(ParseState::ReadingArray { elements, .. }) =
                                self.nested_stack.last_mut()
                            {
                                elements.push(completed_array);
                            }
                        }
                    }
                    if !value.is_none() {
                        self.buffer.clear();
                        self.state = ParseState::Index { pos: 0 };
                    }
                    return Ok(Some(value));
                }
                ParseState::Error(error) => {
                    self.state = ParseState::Index { pos: 0 };
                    return Err(error);
                }
                _ => self.state = next_state,
            }
        }
    }
}
