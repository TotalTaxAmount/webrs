use std::{collections::HashMap, io::Read};

use flate2::{bufread::GzEncoder, Compression};
use get::handle_get;
use log::{info, trace, warn};
use options::handle_options;

use crate::{api::api::Api, Request, Response, WebrsHttp};

pub mod get;
pub mod options;

pub struct Handlers {}

impl<'a> Handlers {
  pub fn handle_compression(
    server: &WebrsHttp,
    req: Request<'a>,
    mut buf: Vec<u8>,
  ) -> (Vec<u8>, Option<&'a str>) {
    if !req.get_headers().contains_key("accept-encoding") {
      info!(
        "[Request {}] Request does not support compression",
        req.get_id()
      );
      return (buf, None);
    }

    let mut compression_types: Vec<&str> = req
      .get_headers()
      .get("accept-encoding")
      .unwrap()
      .split(", ")
      .collect();

    let mut algorithm = None;

    trace!("{:?}", compression_types);

    let order = ["zstd", "br", "gzip"]; // Compression preference order
    let order_map: HashMap<&str, usize> =
      order.into_iter().enumerate().map(|(i, s)| (s, i)).collect();
    compression_types.sort_by_key(|a: &&str| order_map.get(a).copied().unwrap_or(usize::MAX));

    for compression_type in compression_types {
      match compression_type {
        "gzip" if server.compression.2 => {
          algorithm = Some("gzip");

          let mut read_buf: Vec<u8> = Vec::new();
          let mut e = GzEncoder::new(buf.as_slice(), Compression::default());

          let _ = e.read_to_end(&mut read_buf);

          buf = read_buf;
          break;
        }
        "zstd" if server.compression.0 => {
          algorithm = Some("zstd");

          buf = zstd::encode_all(buf.as_slice(), 3).unwrap();
          break;
        }
        "br" if server.compression.1 => {
          algorithm = Some("br");
          let mut read_buf: Vec<u8> = Vec::new();
          let mut e = brotli::CompressorReader::new(buf.as_slice(), 4096, 11, 21);

          let _ = e.read_to_end(&mut read_buf);

          buf = read_buf;
          break;
        }
        _ => {
          warn!(
            "[Request {}] Unsupported compression algorithm '{}'",
            req.get_id(),
            compression_type
          );
        }
      }
    }

    (buf, algorithm)
  }

  pub async fn handle_request<'r>(server: &WebrsHttp, req: Request<'r>) -> Option<Response<'r>> {
    if req.get_endpoint().starts_with("/api") {
      trace!("[Request {}] Passing to api", req.get_id());

      return Api::handle_api_request(server, req).await;
    }

    let res = match req.get_type() {
      crate::ReqTypes::GET => handle_get(server, req),
      crate::ReqTypes::OPTIONS => return handle_options(req),
      crate::ReqTypes::POST => {
        warn!(
          "[Request {}] POST not allowed for non-api methods",
          req.get_id()
        );
        return Some(Response::basic(405, "Method Not Allowed"));
      }
    };

    res
  }
}
