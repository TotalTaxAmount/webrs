use core::{fmt, str};
use std::collections::HashMap;

use log::{error, trace};
use uid::Id;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReqTypes {
    GET,
    POST,
    OPTIONS,
}

#[derive(Debug, Clone)]
pub struct ResError<'r> {
    code: u16,
    description: &'r str,
}

impl<'r> fmt::Display for ResError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Req error! {} {}", self.code, self.description)
    }
}

impl ResError<'_> {
    pub fn get_code(&self) -> u16 {
        self.code
    }

    pub fn get_description(&self) -> &str {
        self.description
    }
}
#[derive(Debug, Clone)]
pub struct Request<'a> {
    req_type: ReqTypes,
    content_type: &'a str,
    endpoint: &'a str,
    data: Vec<u8>,
    headers: HashMap<String, &'a str>,
    url_params: HashMap<&'a str, &'a str>,
    id: Id<Self>,
}

impl fmt::Display for Request<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} {} HTTP/1.1", self.get_type(), self.get_endpoint())?;
        for h in self.headers.clone() {
            writeln!(f, "{}: {}", h.0, h.1)?;
        }
        writeln!(f)?;
        write!(
            f,
            "{}",
            String::from_utf8(self.get_data()).unwrap_or("[Not utf8]".to_string())
        )?;

        return writeln!(f);
    }
}

impl<'a> Request<'a> {
    pub fn parse(request: &'a [u8]) -> Result<Self, ResError> {
        let header_body_split = b"\r\n\r\n";
        let split_index = request
            .windows(header_body_split.len())
            .position(|w| w == header_body_split);

        let (header_bytes, body_bytes) = match split_index {
            Some(i) => (&request[..i], &request[i + header_body_split.len()..]),
            None => {
                error!("Invalid request");
                return Err(ResError {
                    code: 400,
                    description: "Bad Request",
                });
            }
        };
        let header_str: &str = str::from_utf8(&header_bytes).unwrap();
        let parts: Vec<&str> = header_str.split('\n').collect();

        if parts.is_empty() {
            error!("Invalid request");
            return Err(ResError {
                code: 400,
                description: "Bad Request",
            });
        }

        let base: Vec<&str> = parts[0].split(' ').collect();
        if base.len() < 2 {
            error!("Invalid request length");
            trace!("Request string: {}", header_str);
            return Err(ResError {
                code: 400,
                description: "Bad Request",
            });
        }

        let headers: HashMap<String, &str> = parts[1..]
            .into_iter()
            .filter_map(|f| {
                let mut s = f.split(": ");
                if let (Some(key), Some(value)) = (s.next(), s.next()) {
                    Some((key.trim().to_ascii_lowercase(), value.trim()))
                } else {
                    None
                }
            })
            .collect();

        let (endpoint, url_params_str) = base[1].split_once("?").unwrap_or((base[1], ""));
        let url_params: HashMap<&str, &str> = url_params_str
            .split("&")
            .filter_map(|p| {
                let mut s = p.split("=");
                if let (Some(k), Some(v)) = (s.next(), s.next()) {
                    Some((k.trim(), v.trim()))
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            req_type: match base[0] {
                "GET" => ReqTypes::GET,
                "POST" => ReqTypes::POST,
                "OPTIONS" => ReqTypes::OPTIONS,
                _ => {
                    error!("Unknown http method: {}", base[0]);
                    return Err(ResError {
                        code: 501,
                        description: "Not Implemented",
                    });
                }
            },
            endpoint,
            url_params,
            headers: headers.clone(),
            id: Id::new(),
            content_type: headers.get("content-type").or(Some(&"text/plain")).unwrap(),
            data: body_bytes.to_vec(),
        })
    }

    pub fn get_type(&self) -> ReqTypes {
        self.req_type
    }

    pub fn get_content_type(&self) -> &str {
        &self.content_type
    }

    pub fn get_endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn get_headers(&self) -> HashMap<String, &'a str> {
        self.headers.clone()
    }

    pub fn get_url_params(&self) -> HashMap<&'a str, &'a str> {
        self.url_params.clone()
    }

    pub fn get_id(&self) -> usize {
        <Id<Request<'_>> as Clone>::clone(&self.id).get()
    }
}
