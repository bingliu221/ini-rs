use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, Lines},
    path::Path,
    result,
};

type Result<T> = result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Default)]
pub struct Session {
    properties: HashMap<String, String>,
}

impl Session {
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&String> {
        self.properties.get(key.as_ref())
    }
}

#[derive(Debug, Default)]
pub struct Ini {
    sessions: HashMap<String, Session>,
}

impl Ini {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let lines = io::BufReader::new(file).lines();
        Self::parse(lines)
    }

    pub fn session<K: AsRef<str>>(&self, key: K) -> Option<&Session> {
        self.sessions.get(key.as_ref())
    }

    fn parse(lines: Lines<BufReader<File>>) -> Result<Self> {
        let mut ini = Self::default();
        let mut sess_name = Some(String::from(""));
        let mut sess = Some(Session::default());

        for line in lines {
            let line = line?;
            match Self::parse_line(line) {
                ParseLineResult::NewSession { name } => {
                    ini.sessions
                        .insert(sess_name.take().unwrap(), sess.take().unwrap());
                    sess_name.replace(name);
                    sess.replace(Session::default());
                }
                ParseLineResult::EmptyLine => continue,
                ParseLineResult::KeyValue { key, value } => {
                    if let Some(sess) = sess.as_mut() {
                        sess.properties.insert(key, value);
                    }
                }
                ParseLineResult::InvalidFormat => return Err("parse error".into()),
            }
        }
        ini.sessions
            .insert(sess_name.take().unwrap(), sess.take().unwrap());

        for s in &ini.sessions {
            eprintln!("{:?}", s);
        }

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
                    _ => {
                        started = true;
                    }
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
                            '\'' | '"' => return ParseLineResult::InvalidFormat,
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
            return ParseLineResult::InvalidFormat;
        }

        parts.push(part.trim().to_string());

        let parts: Vec<_> = parts.iter().filter(|part| part.len() > 0).collect();

        match parts.len() {
            1 => {
                let part = &parts[0];
                if part.starts_with("[") && part.ends_with("]") {
                    let name = part[1..part.len() - 1].to_string();
                    ParseLineResult::NewSession { name }
                } else {
                    ParseLineResult::InvalidFormat
                }
            }
            3 => {
                if parts[1] != "=" {
                    ParseLineResult::InvalidFormat
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
                        return ParseLineResult::InvalidFormat;
                    }
                }
                ParseLineResult::EmptyLine
            }
        }
    }
}

enum ParseLineResult {
    NewSession { name: String },
    EmptyLine,
    KeyValue { key: String, value: String },
    InvalidFormat,
}
