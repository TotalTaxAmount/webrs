use crate::{Request, Response};

pub mod api;

pub trait Method: Send + Sync {
  fn get_endpoint(&self) -> &str;

  fn handle_get<'s, 'r>(&'s self, req: Request<'r>) -> Option<Response<'r>>
  where
    'r: 's;

  fn handle_post<'s, 'r>(&'s mut self, req: Request<'r>) -> Option<Response<'r>>
  where
    'r: 's;
}
