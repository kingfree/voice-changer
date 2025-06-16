use reqwest::Client;
use reqwest::StatusCode;
use server::app;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn test_endpoints_from_openapi() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = app();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::builder().build().unwrap();

    let resp = client
        .get(format!("http://{}/openapi.json", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let spec: serde_json::Value = resp.json().await.unwrap();

    if spec["paths"].get("/version").is_some() {
        let version = client
            .get(format!("http://{}/version", addr))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(version, "0.1");
    }

    if spec["paths"].get("/ws/audio").is_some() {
        let url = format!("ws://{}/ws/audio", addr);
        let (mut ws, _) = connect_async(&url).await.unwrap();
        ws.close(None).await.unwrap();
    }

    server.abort();
}
