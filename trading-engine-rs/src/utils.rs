use serde::{Serialize, Deserialize};
use serde::de::Error as _;
use tokio_tungstenite::tungstenite::Message;


pub fn serialize_ws_message<T>(model: &T) -> Result<Message, serde_json::Error>
where T: Serialize {
    let serialized = serde_json::to_string(model)?;
    Ok(Message::Text(serialized.into()))
}

pub fn deserialize_ws_response<T>(msg: &Message) -> Result<T, serde_json::Error>
where T: for<'de> Deserialize<'de> {
    match msg {
        Message::Text(text) => {
            let deserialized: T = serde_json::from_str(&text)?;
            Ok(deserialized)
        },
        Message::Binary(bin) => {
            let deserialized: T = serde_json::from_slice(&bin)?;
            Ok(deserialized)
        },
        _ => Err(serde_json::Error::custom("Unsupported message type for deserialization")),
    }
}