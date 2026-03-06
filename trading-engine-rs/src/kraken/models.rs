use serde::{Serialize, Deserialize};

// Generic Structs from the Kraken API
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum RequestParams {
    Subscribe(SubscribeRequestParams),
    Unsubscribe(UnsubscribeRequestParams),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RequestResult {
    Subscribe(SubscribeRequestResult),
    Unsubscribe(UnsubscribeRequestResult),
}

#[derive(Serialize, Debug)]
pub struct Request {
    pub method: String,
    pub params: RequestParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u32>
}

#[derive(Deserialize, Debug)]
pub struct RequestAck {
    pub method: String,
    pub result: RequestResult,
    pub success: bool,
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    pub req_id: Option<u32>
}

#[derive(Deserialize, Debug)]
pub struct DataResponse {
    pub channel: String,
    pub r#type: String,
    pub data: Vec<DataType>
}

// Enums to define new data types
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum SubscribeRequestParams {
    Candle(CandleSubscribeParams),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum SubscribeRequestResult {
    Candle(CandleSubscribeRequestResult),
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum UnsubscribeRequestParams {
    Candle(CandleUnsubscribeRequestParams),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum UnsubscribeRequestResult {
    Candle(CandleUnsubscribeRequestResult),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum DataType {
    Candle(Candle),
}

// 1. Candle Data Types
#[derive(Serialize, Debug)]
pub struct CandleSubscribeParams {
    pub channel: String,
    pub symbol: Vec<String>,
    pub interval: u32,
    pub snapshot: bool,
}

#[derive(Deserialize, Debug)]
pub struct CandleSubscribeRequestResult {
    pub channel: String,
    pub symbol: String,
    pub snapshot: bool,
    pub warnings: Vec<String>
}

#[derive(Serialize, Debug)]
pub struct CandleUnsubscribeRequestParams {
    pub channel: String,
    pub symbol: Vec<String>,
    pub interval: u32,
}

#[derive(Deserialize, Debug)]
pub struct CandleUnsubscribeRequestResult {
    pub channel: String,
    pub symbol: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum CandleResponseType {
    Snapshot,
    Update,
}

#[derive(Deserialize, Debug)]
pub struct Candle {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vwap: f64,
    pub trades: f64,
    pub volume: f64,
    pub interval_begin: String,
    pub interval: i32,
    pub timestamp: String // Deprecated but still used so far...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_subscribe_request_serialization() {
        let req = Request {
            method: String::from("subscribe"),
            params: RequestParams::Subscribe(SubscribeRequestParams::Candle(CandleSubscribeParams {
                channel: String::from("ohlc"),
                symbol: vec![String::from("BTC/USD")],
                interval: 1,
                snapshot: false,
            })),
            req_id: Some(1)
        };
        let serialized = serde_json::to_string(&req).unwrap();
        let json = r#"{"method":"subscribe","params":{"channel":"ohlc","symbol":["BTC/USD"],"interval":1,"snapshot":false},"req_id":1}"#;
        assert_eq!(serialized, json);
    }

    #[test]
    fn test_candle_subscribe_ack_deserialization() {
        let ack = r#"{"method":"subscribe","result":{"channel":"ohlc","symbol":"BTC/USD","snapshot":false,"warnings":[]},"success":true,"time_in":"2024-06-10T12:00:00Z","time_out":"2024-06-10T12:00:01Z","req_id":1}"#;
        let deserialized: RequestAck = serde_json::from_str(ack).unwrap();
        
        assert_eq!(deserialized.method, "subscribe");
        assert_eq!(deserialized.success, true);
        assert!(deserialized.error.is_none());
        
        match deserialized.result {
            RequestResult::Subscribe(SubscribeRequestResult::Candle(result)) => {
                assert_eq!(result.channel, "ohlc");
                assert_eq!(result.symbol, "BTC/USD");
                assert_eq!(result.snapshot, false);
                assert_eq!(result.warnings.len(), 0);
            }
            _ => panic!("Expected Subscribe result"),
        }
    }

    #[test]
    fn test_candle_subscribe_ack_with_error() {
        let ack = r#"{"method":"subscribe","result":{"channel":"ohlc","symbol":"BTC/USD","snapshot":false,"warnings":[]},"success":false,"error":"Invalid symbol","time_in":"2024-06-10T12:00:00Z","time_out":"2024-06-10T12:00:01Z","req_id":1}"#;
        let deserialized: RequestAck = serde_json::from_str(ack).unwrap();
        
        assert_eq!(deserialized.success, false);
        assert_eq!(deserialized.error, Some("Invalid symbol".to_string()));
    }

    #[test]
    fn test_candle_data_response_deserialization() {
        let response = r#"{"channel":"ohlc","type":"update","data":[{"symbol":"BTC/USD","open":61800.0,"high":62299.9,"low":61500.0,"close":61902.1,"trades":833.0,"volume":58.07410209,"vwap":61795.7,"interval_begin":"2026-02-06T00:09:00.000000000Z","interval":1,"timestamp":"2026-02-06T00:10:00.000000Z"}]}"#;
        let deserialized: DataResponse = serde_json::from_str(response).unwrap();
        
        assert_eq!(deserialized.channel, "ohlc");
        assert_eq!(deserialized.r#type, "update");
        assert_eq!(deserialized.data.len(), 1);
        
        match &deserialized.data[0] {
            DataType::Candle(candle) => {
                assert_eq!(candle.symbol, "BTC/USD");
                assert_eq!(candle.open, 61800.0);
                assert_eq!(candle.high, 62299.9);
                assert_eq!(candle.low, 61500.0);
                assert_eq!(candle.close, 61902.1);
            }
        }
    }

    #[test]
    fn test_candle_unsubscribe_request_serialization() {
        let req = Request {
            method: String::from("unsubscribe"),
            params: RequestParams::Unsubscribe(UnsubscribeRequestParams::Candle(CandleUnsubscribeRequestParams {
                channel: String::from("ohlc"),
                symbol: vec![String::from("BTC/USD")],
                interval: 1,
            })),
            req_id: Some(2)
        };
        let serialized = serde_json::to_string(&req).unwrap();
        let json = r#"{"method":"unsubscribe","params":{"channel":"ohlc","symbol":["BTC/USD"],"interval":1},"req_id":2}"#;
        assert_eq!(serialized, json);
    }
}