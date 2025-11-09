//! Connection handling
//!
//! Manages individual client connections, parsing RESP commands
//! and sending responses.

use crate::dispatch::Dispatcher;
use crate::cluster::ClusterManager;
use crate::protocol::{RespParser, RespEncoder, RespValue, RespError};
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Connection handler
pub struct Connection {
    /// TCP stream
    stream: TcpStream,

    /// Read buffer
    read_buffer: BytesMut,

    /// Write buffer
    write_buffer: BytesMut,
}

impl Connection {
    /// Create a new connection handler
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream,
            read_buffer: BytesMut::with_capacity(4096),
            write_buffer: BytesMut::with_capacity(4096),
        }
    }

    /// Handle the connection
    ///
    /// Reads commands from the client, dispatches them, and sends responses.
    pub async fn handle(
        &mut self,
        dispatcher: Arc<Mutex<Dispatcher>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Read data from the socket
            let n = self.stream.read_buf(&mut self.read_buffer).await?;

            // Connection closed
            if n == 0 {
                if self.read_buffer.is_empty() {
                    return Ok(());
                } else {
                    return Err("connection reset by peer".into());
                }
            }

            debug!("Read {} bytes", n);

            // Try to parse commands from the buffer
            loop {
                match RespParser::parse(&mut self.read_buffer) {
                    Ok(Some(value)) => {
                        debug!("Parsed command: {}", value);

                        // Dispatch the command
                        let response = {
                            let mut disp = dispatcher.lock().await;
                            disp.dispatch(value)
                        };

                        debug!("Response: {}", response);

                        // Encode and send the response
                        self.send_response(response).await?;
                    }
                    Ok(None) => {
                        // Need more data
                        debug!("Need more data to complete command");
                        break;
                    }
                    Err(RespError::Incomplete) => {
                        // Need more data
                        debug!("Incomplete command");
                        break;
                    }
                    Err(e) => {
                        // Protocol error
                        warn!("Protocol error: {}", e);
                        let error_response = RespValue::error(format!("ERR protocol error: {}", e));
                        self.send_response(error_response).await?;
                        break;
                    }
                }
            }
        }
    }

    /// Handle the connection with cluster manager
    ///
    /// Reads commands from the client, routes them to appropriate shards, and sends responses.
    pub async fn handle_with_cluster(
        &mut self,
        cluster: Arc<ClusterManager>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Read data from the socket
            let n = self.stream.read_buf(&mut self.read_buffer).await?;

            // Connection closed
            if n == 0 {
                if self.read_buffer.is_empty() {
                    return Ok(());
                } else {
                    return Err("connection reset by peer".into());
                }
            }

            debug!("Read {} bytes", n);

            // Try to parse commands from the buffer
            loop {
                match RespParser::parse(&mut self.read_buffer) {
                    Ok(Some(value)) => {
                        debug!("Parsed command: {}", value);

                        // Execute the command on the cluster
                        let response = cluster.execute(value).await;

                        debug!("Response: {}", response);

                        // Encode and send the response
                        self.send_response(response).await?;
                    }
                    Ok(None) => {
                        // Need more data
                        debug!("Need more data to complete command");
                        break;
                    }
                    Err(RespError::Incomplete) => {
                        // Need more data
                        debug!("Incomplete command");
                        break;
                    }
                    Err(e) => {
                        // Protocol error
                        warn!("Protocol error: {}", e);
                        let error_response = RespValue::error(format!("ERR protocol error: {}", e));
                        self.send_response(error_response).await?;
                        break;
                    }
                }
            }
        }
    }

    /// Send a response to the client
    async fn send_response(&mut self, response: RespValue) -> Result<(), Box<dyn std::error::Error>> {
        // Encode the response
        self.write_buffer.clear();
        RespEncoder::encode_to(&mut self.write_buffer, &response);

        // Write to the socket
        self.stream.write_all(&self.write_buffer).await?;
        self.stream.flush().await?;

        Ok(())
    }
}
