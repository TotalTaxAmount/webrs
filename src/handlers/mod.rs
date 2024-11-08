use std::{collections::HashMap, io::Read, usize};

use flate2::{bufread::GzEncoder, Compression};
use get::handle_get;
use log::{info, warn};

use crate::{api::{api::Api, Method}, Request, Response};

pub mod get;

pub struct Handlers {}

impl<'a> Handlers {
  pub fn handle_encoding(req: Request<'a>, mut buf: Vec<u8>) -> (Vec<u8>, Option<&'a str>) {
    if !req.get_headers().contains_key("Accept-Encoding") {
      info!("[Request {}] Request does not support compression", req.get_id());
      return (buf, None);
    }

    let mut compression_types: Vec<&str> = req.get_headers().get("Accept-Encoding")
      .unwrap()
      .split(", ")
      .collect();

    let mut algorithm = None;

    let order = ["zstd", "br", "gzip"]; // Compression preference order
    let order_map: HashMap<&str, usize> = order.into_iter().enumerate().map(|(i, s)| (s, i)).collect();
    compression_types.sort_by_key(|a: &&str| order_map.get(a).copied().unwrap_or(usize::MAX));

    for compression_type in compression_types {
      match compression_type {
        "gzip" => {
          algorithm = Some(compression_type);

          let mut read_buf: Vec<u8> = Vec::new();
          let mut e = GzEncoder::new(buf.as_slice(), Compression::fast());

          let _ = e.read_to_end(&mut read_buf);

          buf = read_buf;
          break;
        }
        "zstd" => {
          algorithm = Some(compression_type);

          buf = zstd::encode_all(buf.as_slice(), 3).unwrap();
          break;
        }
        "br" => {
          let mut read_buf: Vec<u8> = Vec::new();
          let mut e = brotli::CompressorReader::new(buf.as_slice(), 4096, 11, 21);
          
          let _ = e.read_to_end(&mut read_buf);
          
          buf = read_buf;
        }
        _ => {
          warn!("[Request {}] Unsupported compression algorithm '{}'", req.get_id(), compression_type);
        }
      }
    }

    (buf, algorithm)

  }

  pub fn handle_request(req: Request) -> Option<Response> {
    if req.get_endpoint().starts_with("/api") {
      return Api::handle_api_request(req);
    }

    match req.get_type() {
        crate::ReqTypes::GET => { return handle_get(req); }
        crate::ReqTypes::POST => {
          warn!("[Request {}] POST not allowed for non-api methods", req.get_id());
          return Some(Response::new(405, "text/html").with_description("Method Not Allowed"));
        },
    }
  }
}