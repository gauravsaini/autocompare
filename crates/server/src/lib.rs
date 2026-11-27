//! Localhost HTTP JSON-RPC + static asset server for `autocompare web`.
//!
//! Strictly binds `Ipv4Addr::LOCALHOST` (`127.0.0.1`) with a per-process
//! bearer token (`X-AutoCompare-Token`) and delegates all HTTP/RPC parsing
//! to [`http::handle_http_stream`] and [`rpc::dispatch_rpc`].

#![forbid(unsafe_code)]

pub mod http;
pub mod rpc;

use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use http::{ServerConfig, handle_http_stream};
pub use rpc::{MAX_RPC_BODY_BYTES, RpcRequest, RpcResponse, dispatch_rpc};

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a per-process bearer token without external RNG dependencies.
#[must_use]
pub fn generate_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = u64::from(std::process::id());
    let seq = TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("ac-{pid:x}-{nanos:x}-{seq:x}")
}

/// Runs the HTTP JSON-RPC server on `127.0.0.1:<port>` until terminated.
///
/// # Errors
///
/// Returns an [`io::Error`] if binding the local TCP port fails.
pub fn serve_blocking(mut config: ServerConfig) -> io::Result<()> {
    if config.token.is_empty() {
        config.token = generate_token();
    }
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, config.port));
    let listener = TcpListener::bind(addr)?;
    let bound = listener.local_addr()?;
    let shared_cfg = Arc::new(config);

    eprintln!("AutoCompare Web UI listening on http://{bound}/");
    eprintln!("RPC endpoint: http://{bound}/rpc");
    eprintln!("Session token: {}", shared_cfg.token);

    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        let cfg_clone = Arc::clone(&shared_cfg);
        thread::spawn(move || {
            let _ = handle_connection(stream, &cfg_clone);
        });
    }
    Ok(())
}

fn handle_connection(stream: TcpStream, cfg: &ServerConfig) -> io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    let writer = stream.try_clone()?;
    handle_http_stream(stream, writer, cfg)
}
