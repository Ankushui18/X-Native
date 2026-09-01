//! Minimal recursive-descent JSON parser shared by the .x deserializer
//! and the external-format importers (.sketch). Extracted from
//! deserialize.rs so importers don't grow a second hand-rolled parser.

// ------------------------------------------------------------------- parser

pub(crate) struct P<'a> { s: &'a [u8], i: usize }
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum V { Null, Bool(bool), Num(f64), Str(String), Arr(Vec<V>), Obj(Vec<(String, V)>) }
impl V {
    pub(crate) fn get(&self, key: &str) -> Option<&V> { if let V::Obj(m) = self { m.iter().find(|(k, _)| k == key).map(|(_, v)| v) } else { None } }
    pub(crate) fn str(&self) -> Option<&str> { if let V::Str(s) = self { Some(s) } else { None } }
    pub(crate) fn num(&self) -> Option<f64> { if let V::Num(n) = self { Some(*n) } else { None } }
    pub(crate) fn boolean(&self) -> Option<bool> { if let V::Bool(b) = self { Some(*b) } else { None } }
    pub(crate) fn arr(&self) -> Option<&Vec<V>> { if let V::Arr(a) = self { Some(a) } else { None } }
}

impl<'a> P<'a> {
    pub(crate) fn new(s: &'a str) -> Self { Self { s: s.as_bytes(), i: 0 } }
    fn ws(&mut self) { while self.i < self.s.len() && (self.s[self.i] as char).is_ascii_whitespace() { self.i += 1; } }
    fn peek(&mut self) -> Option<u8> { self.ws(); self.s.get(self.i).copied() }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) { self.i += 1; Ok(()) } else { Err(format!("expected '{}' at {}", c as char, self.i)) }
    }
    pub(crate) fn value(&mut self) -> Result<V, String> {
        match self.peek().ok_or("eof")? {
            b'{' => {
                self.eat(b'{')?;
                let mut m = vec![];
                if self.peek() == Some(b'}') { self.eat(b'}')?; return Ok(V::Obj(m)); }
                loop {
                    let k = match self.value()? { V::Str(s) => s, _ => return Err("key must be string".into()) };
                    self.eat(b':')?;
                    m.push((k, self.value()?));
                    match self.peek() { Some(b',') => { self.eat(b',')?; } _ => break }
                }
                self.eat(b'}')?;
                Ok(V::Obj(m))
            }
            b'[' => {
                self.eat(b'[')?;
                let mut a = vec![];
                if self.peek() == Some(b']') { self.eat(b']')?; return Ok(V::Arr(a)); }
                loop {
                    a.push(self.value()?);
                    match self.peek() { Some(b',') => { self.eat(b',')?; } _ => break }
                }
                self.eat(b']')?;
                Ok(V::Arr(a))
            }
            b'"' => {
                self.eat(b'"')?;
                let mut out = String::new();
                while let Some(&c) = self.s.get(self.i) {
                    self.i += 1;
                    match c {
                        b'"' => return Ok(V::Str(out)),
                        b'\\' => {
                            let e = *self.s.get(self.i).ok_or("eof in escape")?;
                            self.i += 1;
                            match e {
                                b'n' => out.push('\n'), b't' => out.push('\t'), b'r' => out.push('\r'),
                                b'u' => {
                                    let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4).ok_or("bad \\u")?).map_err(|_| "bad utf8")?;
                                    let cp = u32::from_str_radix(hex, 16).map_err(|_| "bad hex")?;
                                    out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                                    self.i += 4;
                                }
                                other => out.push(other as char),
                            }
                        }
                        c => {
                            // re-assemble multi-byte utf8
                            let start = self.i - 1;
                            let len = if c < 0x80 { 1 } else if c >> 5 == 0b110 { 2 } else if c >> 4 == 0b1110 { 3 } else { 4 };
                            let slice = self.s.get(start..start + len).ok_or("bad utf8")?;
                            out.push_str(std::str::from_utf8(slice).map_err(|_| "bad utf8")?);
                            self.i = start + len;
                        }
                    }
                }
                Err("unterminated string".into())
            }
            b't' => { self.i += 4; Ok(V::Bool(true)) }
            b'f' => { self.i += 5; Ok(V::Bool(false)) }
            b'n' => { self.i += 4; Ok(V::Null) }
            _ => {
                self.ws();
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') { self.i += 1; }
                std::str::from_utf8(&self.s[start..self.i]).ok()
                    .and_then(|t| t.parse().ok())
                    .map(V::Num)
                    .ok_or_else(|| format!("bad number at {start}"))
            }
        }
    }
}


/// Parse a complete JSON document.
pub(crate) fn parse(text: &str) -> Result<V, String> { P::new(text).value() }
