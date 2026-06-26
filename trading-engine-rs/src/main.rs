use common::config::load_config;
use common::logging::init_logging;
use serde::{Deserialize};
use tokio::sync::mpsc::{channel, Sender, Receiver};

#[derive(Deserialize, Debug)]
struct AppConfig {
    api_url: String,
}

mod utils;
mod models;
mod orderbook;
mod websocket;
use websocket::kraken_client::KrakenWSClient;
mod kraken;
use crate::kraken::models::DataResponse;

#[tokio::main]
async fn main() {
    init_logging();
    log::info!("Application started");
    let config: AppConfig = load_config().expect("Failed to load configuration");
    log::info!("API URL: {}", config.api_url);


    let (tx, mut rx): (Sender<DataResponse>, Receiver<DataResponse>) = channel(100); // Buffer size of 100 for incoming data responses

    let mut ws_client = KrakenWSClient::new(config.api_url, None);
    ws_client.connect(tx).await.expect("Failed to connect to WebSocket");

    let subscribe_request = kraken::models::Request {
        method: "subscribe".to_string(),
        params: kraken::models::RequestParams::Subscribe(
            kraken::models::SubscribeRequestParams::Candle(
                kraken::models::CandleSubscribeParams {
                    channel: "ohlc".to_string(),
                    symbol: vec!["BTC/USD".to_string()],
                    interval: 1,
                    snapshot: false,
                }
            ),
        ),
        req_id: None,
    };

    ws_client.send_request(subscribe_request).await.expect("Failed to subscribe");

    log::info!("Subscription successful, now handling messages...");

    // Dummy Logging For Now
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            log::info!("Received data: {:?}", data);
        }
    });


    tokio::time::sleep(std::time::Duration::from_secs(60)).await; // Keep the application running for a while to receive messages

    let unsubscribe_request = kraken::models::Request {
        method: "unsubscribe".to_string(),
        params: kraken::models::RequestParams::Unsubscribe(
            kraken::models::UnsubscribeRequestParams::Candle(
                kraken::models::CandleUnsubscribeRequestParams {
                    channel: "ohlc".to_string(),
                    symbol: vec!["BTC/USD".to_string()],
                    interval: 1,
                }
            ),
        ),
        req_id: None,
    };
    ws_client.send_request(unsubscribe_request).await.expect("Failed to unsubscribe");
}