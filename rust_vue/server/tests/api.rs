use reqwest::Client;
use reqwest::StatusCode;
use server::app;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_version_and_openapi() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = app();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::builder().build().unwrap();

    let version = client
        .get(format!("http://{}/version", addr))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(version, "0.1");

    let resp = client
        .get(format!("http://{}/openapi.json", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json["paths"].get("/version").is_some());

    server.abort();
}
