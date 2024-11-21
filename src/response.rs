use core::str;
use std::{collections::HashMap, sync::Arc};

use log::error;
use serde_json::{to_string, Value};
use tokio::{io::AsyncWriteExt, net::tcp::WriteHalf, sync::Mutex};

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
