use async_trait::async_trait;

use crate::{request::Request, response::Response};

pub mod api;

#[async_trait]
pub trait ApiMethod: Send + Sync {
    fn get_endpoint(&self) -> &str;

    async fn handle_get<'s, 'r>(&'s self, req: Request<'r>) -> Option<Response<'r>>
    where
        'r: 's;

    async fn handle_post<'s, 'r>(&'s mut self, req: Request<'r>) -> Option<Response<'r>>
    where
        'r: 's;
}
