use std::{collections::HashMap, io::Read};

use flate2::{bufread::GzEncoder, Compression};
use get::handle_get;
use log::{info, trace, warn};
use options::handle_options;
use tokio::runtime::Handle;

use crate::{api::api::Api, Request, Response, WebrsHttp};

pub mod get;
pub mod options;

pub struct Handlers {}

impl<'a> Handlers {
  pub fn handle_compression(
    server: &WebrsHttp,
    req: Request<'a>,
    mut res: Response<'a>,
  ) -> Response<'a> {
    if !req.get_headers().contains_key("accept-encoding") {
      info!(
        "[Request {}] Request does not support compression",
        req.get_id()
      );
      return res;
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
          let data = res.get_data();
          let mut e = GzEncoder::new(data.as_slice(), Compression::default());

          let _ = e.read_to_end(&mut read_buf);

          res.set_data(read_buf);
          break;
        }
        "zstd" if server.compression.0 => {
          algorithm = Some("zstd");

          res.set_data(zstd::encode_all(res.get_data().as_slice(), 3).unwrap());
          break;
        }
        "br" if server.compression.1 => {
          algorithm = Some("br");
          let mut read_buf: Vec<u8> = Vec::new();
          let data = res.get_data();
          let mut e = brotli::CompressorReader::new(data.as_slice(), 4096, 11, 21);

          let _ = e.read_to_end(&mut read_buf);

          res.set_data(read_buf);
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
    
    if algorithm.is_some() {
      info!("[Request {}] Request is using '{}' compression", req.get_id(), algorithm.unwrap());
      res.add_header(
        "content-encoding".to_string(),
        algorithm.unwrap(),
      );
    }
    
    res
  }

  pub async fn handle_request<'r>(server: &WebrsHttp, req: Request<'r>) -> Option<Response<'r>> {
    let mut res: Option<Response> = if req.get_endpoint().starts_with("/api") {
      trace!("[Request {}] Passing to api", req.get_id());
      Api::handle_api_request(server, req.clone()).await
    } else {
      match req.get_type() {
        crate::ReqTypes::GET => handle_get(server, req.clone()),
        crate::ReqTypes::OPTIONS => handle_options(req.clone()),
        _ => Some(Response::basic(405, "Method Not Allowed"))
      }
    };

    res = Some(Handlers::handle_compression(server, req, res.unwrap()));

    res
  }
}
