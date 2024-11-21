use log::{info, trace, warn};

use crate::{
    api::ApiMethod,
    handlers::Handlers,
    request::Request,
    response::{respond, Response},
};

use std::{net::SocketAddr, process::exit, sync::Arc, thread::AccessError, time::Duration};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::sleep,
};

#[derive(Clone)]
pub struct WebrsHttp {
    api_methods: Arc<Mutex<Vec<Arc<Mutex<dyn ApiMethod + Send + Sync>>>>>,
    port: u16,
    compression: (
        bool, /* zstd */
        bool, /* br */
        bool, /* gzip */
    ),
    content_dir: String,
    running: Arc<Mutex<bool>>,
}

impl WebrsHttp {
    pub fn new(
        port: u16,
        compression: (bool, bool, bool),
        content_dir: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            api_methods: Arc::new(Mutex::new(Vec::new())),
            port,
            compression,
            content_dir,
            running: Arc::new(Mutex::new(true)),
        })
    }

    pub fn get_compression(&self) -> (bool, bool, bool) {
        self.compression
    }

    pub fn get_content_dir(&self) -> String {
        self.content_dir.clone()
    }

    pub async fn get_api_methods(&self) -> Vec<Arc<Mutex<dyn ApiMethod + Send + Sync>>> {
        self.api_methods.lock().await.clone()
    }

    pub async fn register_method(&self, method: Arc<Mutex<dyn ApiMethod + Send + Sync>>) {
      let mut methods = self.api_methods.lock().await;
      methods.push(method);
    }

    pub async fn start(self: Arc<Self>) -> std::io::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Started listening on port {}", self.port);

        tokio::spawn(async move {
            while let Ok((s, a)) = listener.accept().await {
                if !*self.running.lock().await {
                    break;
                }
                let clone = self.clone();

                tokio::spawn(async move {
                    let _ = clone.handle(s, a).await;
                });
            }

            info!("Shutting down web server");
            exit(0)
        });

        Ok(())
    }

    pub async fn stop(&self) {
      let mut running = self.running.lock().await;
      *running = false;
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
