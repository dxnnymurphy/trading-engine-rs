use crate::utils;
use crate::kraken::models::{Request, RequestAck, DataResponse, RequestParams};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::sync::{Mutex, oneshot, mpsc::Sender};
type AckChannel = oneshot::Sender<RequestAck>;
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsReader = SplitStream<WsStream>;
type WsWriter = SplitSink<WsStream, Message>;

const HEARTBEAT_MSG: &str = r#"{"channel":"heartbeat"}"#;
const STREAM_UNITIALISED_ERR: &str = "WebSocket stream not inititalised, call connect() first.";

/// A WebSocket client designed for connecting to the Kraken API.
/// 
/// Provides methods to connect, subscribe and unsubscribe as well as giving the option for an mpsc channel
/// sender to process and send data to other parts of the application.
pub struct KrakenWSClient {
    /// WebSocket URL for the API
    url: String,
    /// WebSocket writer, used to send messages to the API
    write: Option<WsWriter>,
    /// Handle for the background task that routes incoming messages
    task_handle: Option<JoinHandle<()>>,
    /// Request ID tracker used for tracking requests and responses, incremented for each new request
    next_req_id: u32,
    /// HashMap to track pending requests to the api, via request ID
    pending_acks: Arc<Mutex<HashMap<u32, AckChannel>>>
}

impl KrakenWSClient {
    /// Creates a new Kraken WebSocket client.
    /// # Arguments
    /// * `url` - The WebSocket URL for the Kraken API.
    /// * `initial_req_id` - Optional initial request ID, defaults to 0 if not provided.
    pub fn new(url: String, initial_req_id: Option<u32>) -> Self {
        Self { url, write: None, task_handle: None, next_req_id: initial_req_id.unwrap_or(0), pending_acks: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Connects to the Kraken WebSocket API and starts a background task to route incoming messages.
    /// # Arguments
    /// * `tx` - An mpsc channel sender to send data responses to other parts of the application.
    /// # Returns
    /// * `Result<(), Box<dyn std::error::Error>>` - Ok if the connection was successful, Err if an error occurred during connection or message routing.
    pub async fn connect(&mut self, tx: Sender<DataResponse>) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, response) = connect_async(&self.url)
            .await
            .expect(&format!("Failed to connect to url: {}", self.url));
        log::info!("WebSocket connection established: {:?}", response);

        let (write, mut read) = ws_stream.split();
        self.write = Some(write);

        // Kraken sends a welcome message on connection, we can read and ignore it here
        if let Some(message) = read.next().await {
            match message {
                Ok(msg) => log::info!("Received welcome message: {}", msg),
                Err(e) => log::error!("Error receiving message: {}", e),
            }
        }

        let pending_acks_clone = Arc::clone(&self.pending_acks);
        let task_handle = tokio::spawn(async move {
            if let Err(e) = crate::websocket::kraken_client::route_messages(read, pending_acks_clone, tx).await {
                log::error!("Error routing messages: {}", e);
            }
        });

        self.task_handle = Some(task_handle);

        // Start The Route
        Ok(())
    }

    /// Subscribes to a channel via the WebSocket connection, sends a request and waits for an acknowledgement from the API before returning
    /// # Arguments
    /// * `request` - A subscribe request to send to the API, must be of type RequestParams::Subscribe
    /// # Returns
    /// * `Result<(), Box<dyn std::error::Error>>` - Ok if the subscribe request was successful, Err if an error occurred
    pub async fn send_request(&mut self, mut request: Request) -> Result<(), Box<dyn std::error::Error>>
    {
        if request.req_id.is_none() {
            request.req_id = Some(self._get_next_req_id());
        }
        let request_id = request.req_id.unwrap();
        let request_json = utils::serialize_ws_message(&request)?;

        let ack = self._send_request_and_receive_ack(request_id, request_json).await?;

        if !ack.success {
            log::error!("Subscribe failed: {:?}", ack.error);
            return Err("Subscribe failed".into());
        }
        log::info!("Subscribe successful: {:?}", ack.result);
        Ok(())
    }

    /// Helper function to send a request and wait for an acknowledgement from the API
    /// # Arguments
    /// * `request_id` - The ID of the request being sent, used to match
    /// * `request_message` - The message to send to the API
    /// # Returns
    /// * `Result<RequestAck, Box<dyn std::error::Error>>` - Ok with the request acknowledgement if successful, Err if an error occurred during sending or waiting for the acknowledgement
    async fn _send_request_and_receive_ack(&mut self, request_id: u32, request_message: Message) -> Result<RequestAck, Box<dyn std::error::Error>> {
        let (ack_tx, ack_rx) = oneshot::channel();
        {
            let mut pending_acks_lock = self.pending_acks.lock().await;
            pending_acks_lock.insert(request_id, ack_tx);
        }
        self.write.as_mut().expect(STREAM_UNITIALISED_ERR).send(request_message).await?;
        let ack = tokio::time::timeout(std::time::Duration::from_secs(5), ack_rx).await??;
        Ok(ack)
    }

    /// Helper function to get the next request ID and increment the internal counter
    /// # Returns
    /// * `u32` - The next request ID to use for a new request
    fn _get_next_req_id(&mut self) -> u32 {
        let id = self.next_req_id;
        self.next_req_id += 1;
        id
    }
}

/// Function to parse and route responses received from the WebSocket connection
/// # Arguments
/// * `reader` - The WebSocket reader stream to read messages from
/// * `pending_acks` - A shared HashMap of pending request acknowledgements, used to route back to caller
/// * `output_channel` - An mpsc channel sender to send data responses to other parts of the application
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if the reader stream ends gracefully, Err if an error occurs during message processing
pub async fn route_messages(mut reader: WsReader, pending_acks: Arc<Mutex<HashMap<u32, AckChannel>>>, output_channel: Sender<DataResponse>) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(Ok(message)) = reader.next().await {
        match _parse_ws_event(&message) {
            WsMessage::Heartbeat => continue,
            WsMessage::Data(data) => {
                if let Err(e) = output_channel.send(data).await {
                    log::error!("Failed to send data to output channel: {}", e);
                }
            },
            WsMessage::Ack(request_ack) => {
                if let Some(id) = request_ack.req_id {
                    let mut pending_acks_lock = pending_acks.lock().await;
                    if let Some(ack_channel) = pending_acks_lock.remove(&id) {
                        ack_channel.send(request_ack).unwrap_or_else(|e| log::error!("Failed to send request ack to channel: {:?}", e));
                    } else {
                        log::warn!("Received ack with unknown request ID: {}", id);
                    }
                } else {
                    log::warn!("Received ack without request ID: {:?}", request_ack);
                }   
            },
            _ => continue
        }
    }
    log::info!("WebSocket reader stream ended gracefully");
    Ok(())
}

/// Helper function to determine whether a message is a heartbeat message periodically received from the Kraken API.
/// # Arguments
/// * `message` - The WebSocket message to check
/// # Returns
/// * `bool` - True if the message is a heartbeat message, false otherwise
fn _is_heartbeat_message(message: &Message) -> bool {
    if let Message::Text(text) = message {
        text.contains(HEARTBEAT_MSG)
    } else {
        false
    }
}

enum WsMessage {
    Ack(RequestAck),
    Data(DataResponse),
    Heartbeat,
    Unknown
}

/// Helper function to parse a Ws Event and return the correct type and data.
/// # Arguments
/// * `message` - The WebSocket message to check
/// # Returns
/// * `WsMessage` - The parsed WebSocket message enum variant
fn _parse_ws_event(message: &Message) -> WsMessage {
    if let Message::Text(text) = message  && text.contains(HEARTBEAT_MSG) {
        return WsMessage::Heartbeat;
    }

    if let Ok(data)  = utils::deserialize_ws_response::<DataResponse>(&message) {
        return WsMessage::Data(data);
    }

    if let Ok(request_ack) = utils::deserialize_ws_response::<RequestAck>(&message) {
        return WsMessage::Ack(request_ack);
    }

    WsMessage::Unknown
}