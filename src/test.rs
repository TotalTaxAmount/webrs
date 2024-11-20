#[cfg(test)]
mod test {
    use std::sync::{Arc};

    use async_trait::async_trait;
    use tokio::sync::Mutex;

    use crate::{api::ApiMethod, request::{ReqTypes, Request}, response::Response, server::WebrsHttp};

    #[tokio::test]
    async fn test_request_parse_valid() {
        let raw_request = b"GET /test?name=value&foo=bar HTTP/1.1\r\n\
                          Host: localhost\r\n\
                          Content-Type: text/plain\r\n\
                          \r\n\
                          Test body";

        let req = Request::parse(raw_request).unwrap();

        assert_eq!(req.get_type(), ReqTypes::GET);
        assert_eq!(req.get_endpoint(), "/test");
        assert_eq!(req.get_headers().get("host").unwrap(), &"localhost");
        assert_eq!(req.get_content_type(), "text/plain");
        assert_eq!(req.get_data(), b"Test body".to_vec());
        assert_eq!(req.get_url_params().get("name").unwrap(), &"value");
        assert_eq!(req.get_url_params().get("foo").unwrap(), &"bar");
    }

    #[tokio::test]
    async fn test_request_parse_invalid() {
        let raw_request = b"INVALID /test HTTP/1.1\r\n\r\n";

        let err = Request::parse(raw_request).unwrap_err();
        assert_eq!(err.get_code(), 501);
        assert_eq!(err.get_description(), "Not Implemented");
    }

    #[tokio::test]
    async fn test_response_basic() {
        let res = Response::basic(404, "Not Found");

        assert_eq!(res.get_code(), 404);
        assert_eq!(res.get_content_type(), "text/html");
        assert!(String::from_utf8(res.get_data())
            .unwrap()
            .contains("404 Not Found"));
    }

    #[tokio::test]
    async fn test_response_from_json() {
        use serde_json::json;

        let json_data = json!({ "key": "value" });
        let res = Response::from_json(200, json_data).unwrap();

        assert_eq!(res.get_code(), 200);
        assert_eq!(res.get_content_type(), "application/json");
        assert!(String::from_utf8(res.get_data())
            .unwrap()
            .contains("\"key\":\"value\""));
    }

    #[tokio::test]
    async fn test_add_api() {
      struct TestMethod {
        test: u16
      }

      #[async_trait]
      impl ApiMethod for TestMethod {
        fn get_endpoint(&self) -> &str {
          "/test"
        }

        async fn handle_get<'s, 'r>(&'s mut self, req: Request<'r>) -> Option<Response<'r>>
        where
            'r: 's {
              None
            }
    
        async fn handle_post<'s, 'r>(&'s mut self, req: Request<'r>) -> Option<Response<'r>>
        where
            'r: 's {
              None
            }
      }

      let method = TestMethod {
        test: 0
      };

      let test_server = WebrsHttp::new(vec![Arc::new(Mutex::new(method))], 8080, (true, true, true), "".to_string());
      test_server.start().await.unwrap();
    }