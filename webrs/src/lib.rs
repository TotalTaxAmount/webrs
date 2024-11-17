pub mod api;
pub mod handlers;

use core::{fmt, str};
use std::{collections::HashMap, fmt::Display, net::SocketAddr, sync::Arc, time::Duration};

use api::Method;
use handlers::Handlers;
use log::{error, info, trace, warn};
use serde_json::{to_string, Value};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{tcp::WriteHalf, TcpListener, TcpStream},
  sync::Mutex,
  time::sleep,
};
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
  id: Id<Self>,
}

#[derive(Debug, Clone)]
pub struct Response<'a> {
  code: u16,
  content_type: String,
  data: Vec<u8>,
  headers: HashMap<String, &'a str>,
  // id: Id<Self>,
}

impl<'a> Response<'a> {
  pub fn new(code: u16, content_type: &'a str) -> Self {
    Self {
      code,
      content_type: content_type.to_string(),
      data: Vec::new(),
      headers: HashMap::new(),
      // id: Id::new(),
    }
  }

  pub fn set_data(&mut self, data: Vec<u8>) {
    self.data = data;
  }

  pub fn get_data(&self) -> Vec<u8> {
    self.data.clone()
  }

  pub fn set_data_as_slice(&mut self, data: &[u8]) {
    self.data = data.to_vec();
  }

  pub fn add_header(&mut self, k: String, v: &'a str) {
    self.headers.insert(k, v);
  }

  pub fn set_code(&mut self, code: u16) {
    self.code = code;
  }

  pub fn set_content_type(&mut self, content_type: String) {
    self.content_type = content_type;
  }

  pub fn get_code(&self) -> u16 {
    self.code
  }

  pub fn get_content_type(&self) -> String {
    self.content_type.clone()
  }

  pub fn get_headers(&self) -> HashMap<String, &'a str> {
    self.headers.clone()
  }

  pub fn basic(code: u16, description: &str) -> Self {
    let http = format!(
      "
      <html>
        <body>
          <h1>{} {}</h1>
        <body>
      </html>
    ",
      code, description
    );

    let mut res = Self::new(code, "text/html");
    res.set_data(http.as_bytes().to_vec());

    res
  }

  pub fn from_json(code: u16, json: Value) -> Result<Self, serde_json::Error> {
    let mut res = Self::new(code, "application/json");
    let json_string = match to_string(&json) {
      Ok(s) => s,
      Err(e) => {
        error!("Failed to stringify json: {}", e);
        return Err(e);
      }
    };

    res.set_data(json_string.into_bytes());

    Ok(res)
  }
}

impl Display for Request<'_> {
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
      endpoint: base[1],
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

  pub fn get_id(&self) -> usize {
    <Id<Request<'_>> as Clone>::clone(&self.id).get()
  }
}

pub async fn respond(stream: Arc<Mutex<WriteHalf<'_>>>, mut res: Response<'_>) {
  let mut stream = stream.lock().await;
  let mut data = format!("HTTP/1.1 {} OK\r\n", res.code).as_bytes().to_vec();

  if !res.headers.contains_key("content-type") {
    res
      .headers
      .insert("content-type".to_string(), res.content_type.as_str());
  }

  if !res.headers.contains_key("content-length") {
    let dl = res.data.len().to_string();
    res
      .headers
      .insert("content-length".to_string(), Box::leak(dl.into_boxed_str()));
  }

  for (k, v) in res.headers {
    let h = format!("{}: {}\r\n", k, v);
    data.extend_from_slice(&h.as_bytes());
  }

  data.extend_from_slice(&b"\r\n".to_vec());
  data.extend_from_slice(&res.data);

  let _ = stream.write_all(&data).await;
  let _ = stream.flush().await;
}

#[derive(Clone)]
pub struct WebrsHttp {
  api_methods: Vec<Arc<Mutex<dyn Method + Send + Sync>>>,
  port: u16,
  compression: (
    bool, /* zstd */
    bool, /* br */
    bool, /* gzip */
  ),
  content_dir: String,
}

impl WebrsHttp {
  pub fn new(
    api_methods: Vec<Arc<Mutex<dyn Method + Send + Sync>>>,
    port: u16,
    compression: (bool, bool, bool),
    content_dir: String,
  ) -> Arc<Self> {
    Arc::new(Self {
      api_methods,
      port,
      compression,
      content_dir,
    })
  }

  pub async fn start(self: Arc<Self>) -> std::io::Result<()> {
    if let Err(_) = std::env::var("SERVER_LOG") {
      std::env::set_var("SERVER_LOG", "info");
    }

    pretty_env_logger::formatted_timed_builder()
      .parse_env("SERVER_LOG")
      .format_timestamp_millis()
      .init();

    let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
    info!("Started listening on port {}", self.port);

    while let Ok((s, a)) = listener.accept().await {
      let clone = Arc::clone(&self);

      tokio::spawn(async move {
        let _ = clone.handle(s, a).await;
      });
    }

    Ok(())
  }

  async fn handle<'a>(
    &'a self,
    mut stream: TcpStream,
    addr: SocketAddr,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let (mut r_stream, w_stream) = stream.split();
    let w_stream = Arc::new(Mutex::new(w_stream));

    loop {
      let mut raw: Vec<u8> = Vec::new();
      let mut buf: [u8; 4096] = [0; 4096];
      while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
        let len = match r_stream.read(&mut buf).await {
          Ok(0) => return Ok(()),
          Ok(len) => len,
          Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            warn!("Would block, retrying...");
            sleep(Duration::from_secs(5)).await;
            continue;
          }
          Err(e) => {
            warn!("Read error: {}", e);
            break;
          }
        };

        raw.extend_from_slice(&buf[..len]);
      }

      let req: Request = match Request::parse(raw.as_slice()) {
        Ok(r) => r,
        Err(e) => {
          respond(
            w_stream.clone(),
            Response::basic(e.get_code(), e.get_description()),
          )
          .await;
          continue;
        }
      };

      let req_id = req.get_id();

      info!(
        "[Request {}] from {}: {:?} {} HTTP/1.1",
        req_id,
        addr.ip(),
        req.get_type(),
        req.get_endpoint()
      );

      let res = Handlers::handle_request(self, req.clone()).await;

      if let Some(r) = res {
        respond(w_stream.clone(), r).await;
      } else {
        warn!("[Request {}] No response", req_id);
        respond(w_stream.clone(), Response::basic(400, "Bad Request")).await;
      }

      if let Some(c) = req.get_headers().get("connection") {
        if c.to_ascii_lowercase() != "keep-alive" {
          trace!("[Request {}] Connection: {}", req_id, c);
          break;
        }
      } else {
        trace!("[Request {}] No connection header", req_id);
        break;
      }
    }

    trace!("Connection to {} closed", addr.ip());

    Ok(())
  }
}
