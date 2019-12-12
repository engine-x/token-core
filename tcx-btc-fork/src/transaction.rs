#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Utxo {
    #[prost(string, tag="1")]
    pub tx_hash: std::string::String,
    #[prost(int32, tag="2")]
    pub vout: i32,
    #[prost(int64, tag="3")]
    pub amount: i64,
    #[prost(string, tag="4")]
    pub address: std::string::String,
    #[prost(string, tag="5")]
    pub script_pub_key: std::string::String,
    #[prost(string, tag="6")]
    pub derived_path: std::string::String,
    #[prost(int64, tag="7")]
    pub sequence: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BtcForkTxInput {
    #[prost(string, tag="1")]
    pub to: std::string::String,
    #[prost(int64, tag="2")]
    pub amount: i64,
    #[prost(message, repeated, tag="3")]
    pub unspents: ::std::vec::Vec<Utxo>,
    #[prost(string, tag="4")]
    pub memo: std::string::String,
    #[prost(int64, tag="5")]
    pub fee: i64,
    #[prost(uint32, tag="6")]
    pub change_idx: u32,
    #[prost(string, tag="7")]
    pub change_address: std::string::String,
    #[prost(string, tag="8")]
    pub network: std::string::String,
    #[prost(string, tag="9")]
    pub seg_wit: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BtcForkSignedTxOutput {
    #[prost(string, tag="1")]
    pub signature: std::string::String,
    #[prost(string, tag="2")]
    pub tx_hash: std::string::String,
}
