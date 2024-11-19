use std::{fs::File, io::Read};

use log::{error, trace, warn};

use crate::{ReqTypes, Request, Response, WebrsHttp};

use super::Handlers;

pub fn handle_get<'a, 'b>(server: &'a WebrsHttp, req: Request<'b>) -> Option<Response<'b>> {
  if req.get_type() != ReqTypes::GET {
    warn!(
      "[Request {}] Request method is {:?} not GET",
      req.get_id(),
      req.get_type()
    );
    return None;
  }

  let mut path: String = req.get_endpoint().to_string();

  if path.ends_with('/') {
    path.push_str("index");
  }

  let content_type = if let Some(dot_pos) = path.rfind('.') {
    &path[(dot_pos + 1)..]
  } else {
    "html"
  };
  let f_name = format!(
    "{}.{}",
    &path[0..path.find(".").unwrap_or(path.len())],
    content_type
  );

  let mut f = File::open(format!("./{}/{}", server.content_dir, f_name));
  let mut res = Response::new(200, "text/html");

  let mime_type = Box::leak(match mime_guess::from_path(f_name.clone()).first() {
    Some(t) => t.essence_str().to_string().into_boxed_str(),
    None => "text/plain".to_string().into_boxed_str(),
  });

  match &mut f {
    Ok(f) => {
      let mut res_data: Vec<u8> = vec![];

      let _ = f.read_to_end(&mut res_data);

      res.set_code(200);
      res.set_content_type(mime_type.to_string());
      res.set_data(res_data);
    }
    Err(_) => {
      error!("404 {} not found", path);
      res.set_code(404);
      res.set_data(
        "
      <html>
        <body>
          <h1>404 Not found</h1>
        </body>
      </html>"
          .as_bytes()
          .to_vec(),
      );
    }
  }

  Some(res)
}
