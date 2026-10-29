//! Line-level G-code parsing: words, comments, parameters and expressions.
//!
//! The dialect follows Mach3 (which follows RS274/NGC): case-insensitive,
//! whitespace is ignored outside comments, `( )` and `;` comments, `/` block
//! delete, `#n` parameters, `#n=expr` assignments and `[ ]` expressions.

use crate::params::Params;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    pub letter: char,
    pub value: f64,
}

/// One parsed line of G-code.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    pub block_delete: bool,
    pub line_number: Option<u32>,
    /// `O` program/subprogram label on this line.
    pub label: Option<u32>,
    pub words: Vec<Word>,
    /// `#n = value` assignments, applied after the line's words are read.
    pub assignments: Vec<(u32, f64)>,
    pub comments: Vec<String>,
}

impl Block {
    pub fn get(&self, letter: char) -> Option<f64> {
        self.words
            .iter()
            .find(|w| w.letter == letter)
            .map(|w| w.value)
    }

    pub fn has(&self, letter: char) -> bool {
        self.words.iter().any(|w| w.letter == letter)
    }

    /// G or M codes on this line, as code*10 (so G38.2 is 382 and G1 is 10).
    pub fn codes(&self, letter: char) -> impl Iterator<Item = u32> + '_ {
        self.words
            .iter()
            .filter(move |w| w.letter == letter)
            .map(|w| (w.value * 10.0).round() as u32)
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.assignments.is_empty() && self.label.is_none()
    }
}

/// Parse one line. Parameter reads are evaluated against `params`.
pub fn parse_line(line: &str, params: &Params) -> Result<Block, String> {
    let mut block = Block::default();
    let mut code = String::with_capacity(line.len());

    // Strip comments first so their text keeps its case and spacing.
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => {
                let mut text = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == ')' {
                        closed = true;
                        break;
                    }
                    text.push(c);
                }
                if !closed {
                    return Err("unclosed comment".into());
                }
                block.comments.push(text.trim().to_string());
            }
            ';' => {
                let text: String = chars.by_ref().collect();
                block.comments.push(text.trim().to_string());
            }
            c if c.is_whitespace() => {}
            c => code.push(c.to_ascii_uppercase()),
        }
    }

    let mut p = Cursor {
        s: code.as_bytes(),
        i: 0,
        params,
    };
    if p.peek() == Some(b'/') {
        block.block_delete = true;
        p.i += 1;
    }
    if p.peek() == Some(b'%') {
        return Ok(block);
    }

    while let Some(c) = p.peek() {
        match c {
            b'#' => {
                p.i += 1;
                let index = p.primary()?;
                if p.peek() != Some(b'=') {
                    return Err("expected '=' after parameter".into());
                }
                p.i += 1;
                let value = p.real_value()?;
                block.assignments.push((param_index(index)?, value));
            }
            b'A'..=b'Z' => {
                p.i += 1;
                let letter = c as char;
                let value = p.real_value()?;
                match letter {
                    'N' => block.line_number = Some(value as u32),
                    'O' => block.label = Some(value as u32),
                    _ => block.words.push(Word { letter, value }),
                }
            }
            _ => return Err(format!("unexpected character '{}'", c as char)),
        }
    }
    Ok(block)
}

fn param_index(v: f64) -> Result<u32, String> {
    if v < 0.0 || v.fract() != 0.0 {
        return Err(format!("invalid parameter number {v}"));
    }
    Ok(v as u32)
}

struct Cursor<'a> {
    s: &'a [u8],
    i: usize,
    params: &'a Params,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, token: &str) -> bool {
        if self.s[self.i..].starts_with(token.as_bytes()) {
            self.i += token.len();
            true
        } else {
            false
        }
    }

    /// A word value: number, parameter, bracketed expression, or a signed one of those.
    fn real_value(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Ok(-self.real_value()?)
            }
            Some(b'+') => {
                self.i += 1;
                self.real_value()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'[') => {
                self.i += 1;
                let v = self.expr()?;
                self.close()?;
                Ok(v)
            }
            Some(b'#') => {
                self.i += 1;
                let index = param_index(self.primary()?)?;
                Ok(self.params.get(index))
            }
            Some(b'-') | Some(b'+') => self.real_value(),
            Some(c) if c.is_ascii_digit() || c == b'.' => self.number(),
            Some(b'A'..=b'Z') => self.function(),
            Some(c) => Err(format!("unexpected '{}' in value", c as char)),
            None => Err("missing value".into()),
        }
    }

    fn close(&mut self) -> Result<(), String> {
        if self.peek() == Some(b']') {
            self.i += 1;
            Ok(())
        } else {
            Err("expected ']'".into())
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == b'.') {
            self.i += 1;
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        text.parse::<f64>()
            .map_err(|_| format!("bad number '{text}'"))
    }

    fn function(&mut self) -> Result<f64, String> {
        const NAMES: [&str; 14] = [
            "ABS", "ACOS", "ASIN", "ATAN", "COS", "EXP", "FIX", "FUP", "LN", "ROUND", "SIN",
            "SQRT", "TAN", "EXISTS",
        ];
        let name = NAMES
            .iter()
            .find(|n| self.s[self.i..].starts_with(n.as_bytes()))
            .ok_or_else(|| "unknown function or misplaced letter in expression".to_string())?;
        self.i += name.len();
        if self.peek() != Some(b'[') {
            return Err(format!("{name} needs [ ]"));
        }
        let a = self.primary()?;
        let r = match *name {
            "ABS" => a.abs(),
            "ACOS" => a.acos().to_degrees(),
            "ASIN" => a.asin().to_degrees(),
            "ATAN" => {
                // ATAN[y]/[x]
                if self.peek() != Some(b'/') {
                    return Err("ATAN needs the form ATAN[y]/[x]".into());
                }
                self.i += 1;
                let b = self.primary()?;
                a.atan2(b).to_degrees()
            }
            "COS" => a.to_radians().cos(),
            "EXP" => a.exp(),
            "FIX" => a.floor(),
            "FUP" => a.ceil(),
            "LN" => a.ln(),
            "ROUND" => a.round(),
            "SIN" => a.to_radians().sin(),
            "SQRT" => a.sqrt(),
            "TAN" => a.to_radians().tan(),
            _ => unreachable!(),
        };
        Ok(r)
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        loop {
            if self.eat("+") {
                v += self.term()?;
            } else if self.eat("-") {
                v -= self.term()?;
            } else if self.eat("OR") {
                v = ((v != 0.0) || (self.term()? != 0.0)) as u8 as f64;
            } else if self.eat("XOR") {
                v = ((v != 0.0) ^ (self.term()? != 0.0)) as u8 as f64;
            } else if self.eat("AND") {
                v = ((v != 0.0) && (self.term()? != 0.0)) as u8 as f64;
            } else {
                return Ok(v);
            }
        }
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.power()?;
        loop {
            if self.eat("*") && !self.s[..self.i].ends_with(b"**") {
                v *= self.power()?;
            } else if self.eat("/") {
                v /= self.power()?;
            } else if self.eat("MOD") {
                v = v.rem_euclid(self.power()?);
            } else {
                return Ok(v);
            }
        }
    }

    fn power(&mut self) -> Result<f64, String> {
        let base = self.real_value()?;
        if self.eat("**") {
            Ok(base.powf(self.power()?))
        } else {
            Ok(base)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Block {
        parse_line(s, &Params::default()).unwrap()
    }

    #[test]
    fn words_and_comments() {
        let b = parse("n10 g1 x1.5 Y-2 (Cut Pocket) f300 ; trailing");
        assert_eq!(b.line_number, Some(10));
        assert_eq!(b.get('G'), Some(1.0));
        assert_eq!(b.get('X'), Some(1.5));
        assert_eq!(b.get('Y'), Some(-2.0));
        assert_eq!(b.get('F'), Some(300.0));
        assert_eq!(b.comments, vec!["Cut Pocket", "trailing"]);
    }

    #[test]
    fn spaces_inside_numbers_are_ignored() {
        let b = parse("G 0 X 1 0 . 5");
        assert_eq!(b.get('X'), Some(10.5));
    }

    #[test]
    fn decimal_codes() {
        let b = parse("G38.2 Z-10 F50");
        assert_eq!(b.codes('G').collect::<Vec<_>>(), vec![382]);
    }

    #[test]
    fn expressions_and_params() {
        let mut params = Params::default();
        params.set(100, 4.0);
        let b = parse_line(
            "#101=[#100*2+1] X[#100/2] Y-#100 Z[2**3] A[7 MOD 4]",
            &params,
        )
        .unwrap();
        assert_eq!(b.assignments, vec![(101, 9.0)]);
        assert_eq!(b.get('X'), Some(2.0));
        assert_eq!(b.get('Y'), Some(-4.0));
        assert_eq!(b.get('Z'), Some(8.0));
        assert_eq!(b.get('A'), Some(3.0));
    }

    #[test]
    fn functions() {
        let b = parse("X[SIN[90]] Y[ATAN[1]/[1]] Z[ABS[-3]]");
        assert!((b.get('X').unwrap() - 1.0).abs() < 1e-12);
        assert!((b.get('Y').unwrap() - 45.0).abs() < 1e-12);
        assert_eq!(b.get('Z'), Some(3.0));
    }

    #[test]
    fn block_delete_and_label() {
        let b = parse("/G0 X1");
        assert!(b.block_delete);
        let b = parse("O1000");
        assert_eq!(b.label, Some(1000));
    }

    #[test]
    fn errors() {
        assert!(parse_line("G1 X1 (open", &Params::default()).is_err());
        assert!(parse_line("G1 X$", &Params::default()).is_err());
    }
}
