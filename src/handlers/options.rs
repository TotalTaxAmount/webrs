use crate::{request::Request, response::Response};

pub fn handle_options(_req: Request) -> Option<Response> {
  let mut res = Response::new(204, "No Content");
  res.add_header("allow".to_string(), "GET, POST, OPTIONS");
  Some(res)
}
