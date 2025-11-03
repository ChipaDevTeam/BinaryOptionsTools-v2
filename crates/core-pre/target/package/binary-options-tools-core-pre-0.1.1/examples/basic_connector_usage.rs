// use core_pre::connector::{BasicConnector, Connector, ConnectorConfig};
// use std::time::Duration;
// use tokio_tungstenite::tungstenite::Message;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Create a connector with custom configuration
//     let config = ConnectorConfig {
//         url: "wss://echo.websocket.org".to_string(),
//         max_reconnect_attempts: 3,
//         reconnect_delay: Duration::from_secs(2),
//         connection_timeout: Duration::from_secs(5),
//         ..Default::default()
//     };

//     let mut connector = BasicConnector::new(config);

//     // Connect to the WebSocket and get the stream
//     println!("Connecting to WebSocket...");
//     let mut stream = match connector.connect().await {
//         Ok(stream) => {
//             println!("Successfully connected!");
//             stream
//         }
//         Err(e) => {
//             eprintln!("Failed to connect: {}", e);
//             return Err(e.into());
//         }
//     };

//     // Send a test message using the helper method
//     let test_message = Message::text("Hello, WebSocket!");
//     println!("Sending message: {:?}", test_message);

//     if let Err(e) = BasicConnector::send_message_to_stream(&mut stream, test_message).await {
//         eprintln!("Failed to send message: {}", e);
//     }

//     // Try to receive a message (echo server should echo back our message)
//     println!("Waiting for response...");
//     match BasicConnector::receive_message_from_stream(&mut stream).await {
//         Ok(Some(message)) => println!("Received: {:?}", message),
//         Ok(None) => println!("Connection closed"),
//         Err(e) => eprintln!("Error receiving message: {}", e),
//     }

//     // Check connection state
//     let state = connector.connection_state();
//     println!("Connection state: {:?}", state);

//     // Close the current stream
//     if let Err(e) = BasicConnector::close_stream(&mut stream).await {
//         eprintln!("Error closing stream: {}", e);
//     }

//     // Test reconnection and get a new stream
//     println!("Testing reconnection...");
//     match connector.reconnect().await {
//         Ok(_new_stream) => {
//             println!("Reconnection successful! Got new stream");
//             // You can now use _new_stream for further operations
//         }
//         Err(e) => eprintln!("Reconnection failed: {}", e),
//     }

//     // Disconnect the connector
//     connector.disconnect().await?;
//     println!("Disconnected successfully");

//     Ok(())
// }
fn main() {}
