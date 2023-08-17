use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::Path,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub type Session = HashMap<String, String>;
pub type Ini = HashMap<String, Session>;

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Ini> {
    let file = File::open(path)?;
    let lines = BufReader::new(file).lines();
    parse_lines(lines)
}

fn parse_lines(lines: Lines<BufReader<File>>) -> Result<Ini> {
    let mut ini = Ini::default();
    let mut sess_name = Some(String::from(""));
    let mut sess = Some(Session::default());

    for (i, line) in lines.enumerate() {
        let line = line?;
        match parse_line(line) {
            ParseLineResult::NewSession { name } => {
                ini.insert(sess_name.take().unwrap(), sess.take().unwrap());
                sess_name.replace(name);
                sess.replace(Session::default());
            }
            ParseLineResult::EmptyLine => continue,
            ParseLineResult::KeyValue { key, value } => {
                if let Some(sess) = sess.as_mut() {
                    sess.insert(key, value);
                }
            }
            ParseLineResult::ParseError { error } => {
                return Err(format!("parse error, line {}, {}", i + 1, error).into())
            }
        }
    }
    ini.insert(sess_name.take().unwrap(), sess.take().unwrap());

    Ok(ini)
}

fn parse_line(line: String) -> ParseLineResult {
    let mut escape = false;
    let mut qoute = Option::<char>::None;
    let mut started = false;

    let mut parts = Vec::<String>::new();
    let mut part = String::default();

    for ch in line.chars() {
        if !started {
            match ch {
                ' ' => continue,
                '\'' | '"' => {
                    qoute = Some(ch);
                    started = true;
                    continue;
                }
                _ => started = true,
            }
        }

        match qoute {
            None => {
                if escape {
                    match ch {
                        '\\' | '\'' | '"' | ';' | '#' | '=' => part.push(ch),
                        _ => {
                            part.push('\\');
                            part.push(ch);
                        }
                    }
                    escape = false;
                } else {
                    match ch {
                        '\\' => escape = true,
                        '=' => {
                            parts.push(part.trim().to_string());
                            parts.push(String::from("="));
                            part.clear();
                            started = false;
                            qoute = None;
                        }
                        ';' | '#' => break,
                        '\'' | '"' => {
                            return ParseLineResult::ParseError {
                                error: "invalid qoute position".into(),
                            }
                        }
                        _ => part.push(ch),
                    }
                }
            }
            Some(q) => {
                if escape {
                    if ch == q {
                        part.push(ch);
                    } else {
                        part.push('\\');
                        part.push(ch);
                    }
                    escape = false;
                } else {
                    if ch == q {
                        started = false;
                        qoute = None;
                    } else {
                        if ch == '\\' {
                            escape = true;
                        } else {
                            part.push(ch);
                        }
                    }
                }
            }
        }
    }

    if qoute.is_some() {
        return ParseLineResult::ParseError {
            error: "unclosed qoute".into(),
        };
    }

    parts.push(part.trim().to_string());

    let parts: Vec<_> = parts.iter().filter(|part| part.len() > 0).collect();

    match parts.len() {
        1 => {
            let part = parts[0];
            if only_starts_with(part, "[") && only_ends_with(part, "]") {
                let name = part[1..part.len() - 1].trim().to_string();
                ParseLineResult::NewSession { name }
            } else {
                ParseLineResult::ParseError {
                    error: "invalid session name format".into(),
                }
            }
        }
        3 => {
            if parts[1] != "=" {
                ParseLineResult::ParseError {
                    error: "invalid assignment".into(),
                }
            } else {
                ParseLineResult::KeyValue {
                    key: parts[0].clone(),
                    value: parts[2].clone(),
                }
            }
        }
        _ => {
            for part in parts {
                if part.len() > 0 {
                    return ParseLineResult::ParseError {
                        error: "extra key or value found ".into(),
                    };
                }
            }
            ParseLineResult::EmptyLine
        }
    }
}

enum ParseLineResult {
    NewSession { name: String },
    EmptyLine,
    KeyValue { key: String, value: String },
    ParseError { error: String },
}

fn only_starts_with(s: &str, pat: &str) -> bool {
    s.starts_with(pat) && !s[pat.len()..].contains(pat)
}

fn only_ends_with(s: &str, pat: &str) -> bool {
    s.ends_with(pat) && !s[..s.len() - pat.len()].contains(pat)
}
