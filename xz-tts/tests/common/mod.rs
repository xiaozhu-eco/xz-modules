use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::accept_async;
use xz_tts::credential::{CredentialProvider, ResolvedTtsCredential, StaticCredential};
use xz_tts::error::XzTtsError;

/// Create a static test credential with fake values suitable for mock testing
pub fn create_test_credential() -> Arc<dyn CredentialProvider> {
    Arc::new(StaticCredential::new(
        "test-app",
        "test-token",
        "volc.service_type.10029",
    ))
}

/// Spawn a mock WebSocket server on a random port.
/// Returns (address, shutdown_tx). Call shutdown_tx.send(()) to stop the server.
pub async fn spawn_mock_server() -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let _ = accept_async(stream).await;
                            // Accept and immediately close (basic mock)
                        }
                        Err(_) => break,
                    }
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    (addr, shutdown_tx)
}
