use thiserror::Error;

pub type JitoClientResult<T> = std::result::Result<T, JitoClientError>;

#[derive(Error, Debug)]
pub enum JitoClientError {
    #[error("Latency measure error")]
    MeasureLatencyError,
    #[error("Missing latency for all regions")]
    AllRegionLatencyMissing,
    #[error("DNS resolution failed: {0}")]
    DNSResolution(std::io::Error),
    #[error("Empty DNS resolution result")]
    DNSEmpty,
    #[error("TCP connection failed: {0}")]
    TCPConnect(std::io::Error),
    #[error("Bundle transaction size reached")]
    TooManyTxns,
    #[error("Retry wait parameters invalid")]
    WaitParameterError,
    #[error("Max retries reached")]
    MaxRetriesError,
    #[error("Bincode serialize error: {0}")]
    SerializeError(#[from] bincode::Error),
    #[error("GRPC connect error: {0}")]
    GRPCError(#[from] tonic::transport::Error),
    #[error("Send Error: {0}")]
    SendError(#[from] tonic::Status),
}
