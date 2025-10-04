use crate::errors::{JitoClientError, JitoClientResult};
use crate::grpc::{
    bundle::Bundle,
    searcher::{searcher_service_client::SearcherServiceClient, SendBundleRequest},
};
use crate::nodes::NodeRegion;
use futures_timer::Delay;
use solana_transaction::versioned::VersionedTransaction;
use std::time::Duration;
use tonic::transport::{channel::ClientTlsConfig, Channel, Endpoint};

pub struct JitoClient {
    client: SearcherServiceClient<Channel>,
    endpoint: &'static str,
}
impl JitoClient {
    /// Creates a new gRPC client that dyanmically determines the fastest endpoint to connect to.
    ///
    /// This method measures latency to all available endpoints and selects the one with the lowest response time for optimal performance.
    ///
    /// # Arguments
    /// * `timeout` - Connection and request timeout in seconds. Defaults to 2 seconds if None is passed.
    ///
    /// # Returns
    /// Returns the configured client connected to the fastest endpoint, or an error if region measurement or connection fails.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - Region latency measurement fails
    /// - Connection to the selected endpoint fails
    ///
    /// # Examples
    /// ```rust
    /// //Use default 2-second timeout
    /// let client = JitoClient::new_dynamic_region(None).await?;
    ///
    /// // Use custom 5-second timeout
    /// let client = JitoClient::new_dynamic_region(Some(5)).await?;
    /// ```
    pub async fn new_dynamic_region(timeout: Option<u64>) -> JitoClientResult<Self> {
        let fastest_endpoint = NodeRegion::measure_latency().await?.0.endpoint();
        let timeout_dur = Duration::from_secs(timeout.unwrap_or(2));
        let channel = Endpoint::from_static(fastest_endpoint)
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .tcp_nodelay(true)
            .timeout(timeout_dur)
            .connect_timeout(timeout_dur)
            .connect()
            .await?;

        Ok(Self {
            client: SearcherServiceClient::new(channel),
            endpoint: fastest_endpoint,
        })
    }

    /// Creates a new gRPC client that connects to a specified input endpoint.
    ///
    /// # Arguments
    /// * `endpoint` - The gRPC endpoint URL
    /// * `timeout` - Connection and request timeout in seconds. Defaults to 2 seconds if None is passed.
    ///
    /// # Returns
    /// Returns the configured client connected to the endpoint, or an error if connection fails.
    ///
    /// # Errors
    /// This function will return an error if connection to the selected endpoint fails
    ///
    /// # Examples
    /// ```rust
    /// // Connect with default timeout
    /// let client = JitoClient::new("https://ny.mainnet.block-engine.jito.wtf:443", None).await?;
    ///
    /// // Connect with custom 10-second timeout
    /// let client = JitoClient::new("https://ny.mainnet.block-engine.jito.wtf:443", Some(10)).await?;
    /// ```
    pub async fn new(endpoint: &'static str, timeout: Option<u64>) -> JitoClientResult<Self> {
        let timeout_dur = Duration::from_secs(timeout.unwrap_or(2));
        let channel = Endpoint::from_shared(endpoint)?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .tcp_nodelay(true)
            .timeout(timeout_dur)
            .connect_timeout(timeout_dur)
            .connect()
            .await?;

        let client = SearcherServiceClient::new(channel);

        Ok(Self { client, endpoint })
    }

    /// Sends a bundle of transactions to the node via gRPC.
    ///
    /// # Arguments
    /// * `transactions` - A vec of transactions (`VersionedTransaction`) to be sent
    ///
    /// # Returns
    /// Returns a String containing the unique bundle ID.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - Too many transactions provided
    /// - Transaction serialization fails
    /// - gRPC connection fails
    /// - Node server returns an error
    ///
    /// # Examples
    /// ```rust
    /// let mut client = JitoClient::new_dynamic_region(None).await?;
    ///
    /// let transactions = vec![];
    ///
    /// match client.send(transactions).await {
    ///     Ok(uuid) => println!("Bundle ID: {}", uuid),
    ///     Err(e) => eprintln!("Failed to send: {}", e),
    /// }
    /// ```
    pub async fn send(
        &mut self,
        transactions: Vec<VersionedTransaction>,
    ) -> JitoClientResult<String> {
        let bundle = Bundle::create(transactions)?;
        let request = SendBundleRequest {
            bundle: Some(bundle),
        };
        let response = self.client.send_bundle(request).await?;
        Ok(response.into_inner().uuid)
    }

    /// Sends a bundle of transactions with automatic retries.
    ///
    /// # Arguments
    /// * `transactions` - A vec of transactions (`VersionedTransaction`) to be sent
    /// * `retry_logic` - Configuration for retry behavior including max attempts and wait times.
    ///
    /// # Returns
    /// Returns a String containing the unique bundle ID.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - Too many transactions provided
    /// - Transaction serialization fails
    /// - gRPC connection fails
    /// - Node server returns an error
    /// - Maximum retry attempts exceeded
    ///
    /// # Retry Behavior
    /// - Uses random jitter between min_wait and max_wait milliseconds
    /// - Logs debug information for each failed attempt
    ///
    /// # Examples
    /// ```rust
    /// let mut client = JitoClient::new_dynamic_region(None).await?;
    /// // 3 retries with default timings
    /// let retry_config = RetryLogic::new(3);     
    ///
    /// let transactions = vec![];
    ///
    /// match client.send_with_retry(transactions, retry_config).await {
    ///     Ok(uuid) => println!("Bundle ID: {}", uuid),
    ///     Err(e) => eprintln!("Failed to send: {}", e),
    /// }
    /// ```
    pub async fn send_with_retry(
        &mut self,
        transactions: Vec<VersionedTransaction>,
        retry_logic: RetryLogic,
    ) -> JitoClientResult<String> {
        let bundle = Bundle::create(transactions)?;
        let request = SendBundleRequest {
            bundle: Some(bundle),
        };
        let mut retries = 0u8;
        loop {
            match self.client.send_bundle(request.clone()).await {
                Ok(response) => {
                    return Ok(response.into_inner().uuid);
                }
                Err(e) => {
                    log::debug!("Send error: {e}");
                    Delay::new(retry_logic.jitter()).await;
                    retries += 1;
                    if retries >= retry_logic.max_retries {
                        return Err(JitoClientError::MaxRetriesError);
                    }
                }
            }
        }
    }

    /// Returns the endpoint URL that this client is currently connected to.
    pub fn get_endpoint(&self) -> &'static str {
        self.endpoint
    }

    /// Returns all available node regions that can be used for connections.
    pub fn all_regions() -> &'static [NodeRegion] {
        NodeRegion::all()
    }
}

pub struct RetryLogic {
    pub max_retries: u8,
    pub min_wait: u64,
    pub max_wait: u64,
}

impl RetryLogic {
    pub fn new(max_retries: u8) -> Self {
        Self {
            max_retries,
            min_wait: 5,
            max_wait: 25,
        }
    }

    pub fn new_with_wait_bounds(
        max_retries: u8,
        min_wait: u64,
        max_wait: u64,
    ) -> JitoClientResult<Self> {
        if min_wait >= max_wait {
            return Err(JitoClientError::WaitParameterError);
        }
        Ok(Self {
            max_retries,
            min_wait,
            max_wait,
        })
    }

    pub fn jitter(&self) -> std::time::Duration {
        std::time::Duration::from_millis(rand::random_range(self.min_wait..=self.max_wait))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use solana_keypair::{Keypair, Signer};
    use solana_program::{
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };
    use solana_system_interface::instruction::transfer;
    use solana_transaction::{Message, VersionedMessage};
    use std::str::FromStr;

    const SERVER_URL1: &str = "https://ny.mainnet.block-engine.jito.wtf:443";
    const SERVER_URL2: &str = "https://ny.testnet.block-engine.jito.wtf:443";

    #[tokio::test]
    #[serial]
    async fn custom_endpoint_default_timeout() {
        match JitoClient::new(SERVER_URL2, None).await {
            Ok(client) => println!("Get Endpoint: {}", client.get_endpoint()),
            Err(e) => panic!("Error in creating client: {e}"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn dynamic_region_custom_timeout() {
        match JitoClient::new_dynamic_region(Some(5)).await {
            Ok(client) => println!("Get Endpoint: {}", client.get_endpoint()),
            Err(e) => panic!("Error in creating client: {e}"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn send_endpoint() {
        let start = std::time::Instant::now();
        let mut client = JitoClient::new(SERVER_URL1, None)
            .await
            .expect("Failed to create client");

        let signer_keypair = Keypair::new();
        let bh = Hash::new_unique();
        let tip_account = Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap();

        let ix = Instruction {
            program_id: Pubkey::from_str("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo").unwrap(),
            accounts: vec![AccountMeta::new(signer_keypair.pubkey(), true)],
            data: b"test".to_vec(),
        };
        let txns = vec![
            ix,
            transfer(&signer_keypair.pubkey(), &tip_account, 100_000),
        ];
        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &txns,
            Some(&signer_keypair.pubkey()),
            &bh,
        ));
        let transaction = VersionedTransaction::try_new(message, &[signer_keypair]).unwrap();

        match client.send(vec![transaction]).await {
            Ok(out) => println!("bundle id: {out}"),
            Err(e) => panic!("Send error: {e}"),
        }
        println!("Elapsed: {} ms", start.elapsed().as_millis());
    }

    #[tokio::test]
    #[serial]
    async fn send_with_retries() {
        let start = std::time::Instant::now();
        let mut client = JitoClient::new(SERVER_URL2, None)
            .await
            .expect("Failed to create client");

        let signer_keypair = Keypair::new();
        let bh = Hash::new_unique();
        let tip_account = Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap();

        let ix = Instruction {
            program_id: Pubkey::from_str("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo").unwrap(),
            accounts: vec![AccountMeta::new(signer_keypair.pubkey(), true)],
            data: b"test".to_vec(),
        };
        let txns = vec![
            ix,
            transfer(&signer_keypair.pubkey(), &tip_account, 100_000),
        ];
        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &txns,
            Some(&signer_keypair.pubkey()),
            &bh,
        ));
        let transaction = VersionedTransaction::try_new(message, &[signer_keypair]).unwrap();

        match client
            .send_with_retry(vec![transaction], RetryLogic::new(3))
            .await
        {
            Ok(out) => println!("bundle id: {out}"),
            Err(e) => println!("Send error: {e}"),
        }
        println!("Elapsed: {} ms", start.elapsed().as_millis());
    }
}
