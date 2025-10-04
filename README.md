# Jito gRPC Client

A Rust client for connecting to Jito's block engine nodes via gRPC with automatic region selection and retry capabilities.

[Check out the full documentation](https://stevenlusonggao.github.io/blog/posts/jito-grpc-client/)

## Features

- **Bundle Transactions**: Send jito bundles via gRPC, no auth key needed
- **Dynamic Region Selection**: Option to automatically connect to the fastest available region based on latency measurements
- **Retry Logic**: Automatic retry with configurable jitter

## Basic Usage Example

```rust
#[tokio::main]
async fn main() -> JitoClientResult<()> {
    // Connect to fastest region automatically
    let mut client = JitoClient::new_dynamic_region(None).await?;
    
    let transactions: Vec<VersionedTransaction> = vec![
        // Your transactions
    ];
    
    // Send bundle
    let uuid = client.send(&transactions).await?;
    println!("Bundle submitted with UUID: {}", uuid);
    
    Ok(())
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
