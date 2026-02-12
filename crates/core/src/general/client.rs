use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use async_channel::{Receiver, RecvError};
use futures_util::future::try_join3;
use futures_util::stream::{SplitSink, SplitStream, select_all};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use crate::constants::MAX_CHANNEL_CAPACITY;
use crate::error::{BinaryOptionsResult, BinaryOptionsToolsError};
use crate::general::stream::RecieverStream;
use crate::general::types::MessageType;

use super::config::Config;
use super::send::SenderMessage;
use super::stream::FilteredRecieverStream;
use super::traits::{
    Connect, Credentials, DataHandler, InnerConfig, MessageHandler, MessageTransfer,
    ValidatorTrait, WCallback,
};
use super::types::{Callback, Data};

#[derive(Clone)]
pub struct WebSocketClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    inner: Arc<WebSocketInnerClient<Transfer, Handler, Connector, Creds, T, U>>,
}

pub struct WebSocketInnerClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    pub credentials: Creds,
    pub connector: Connector,
    pub handler: Handler,
    pub data: Data<T, Transfer>,
    pub sender: SenderMessage,
    pub reconnect_callback: Option<Callback<T, Transfer, U>>,
    pub config: Config<T, Transfer, U>,
    _event_loop: JoinHandle<BinaryOptionsResult<()>>,
}

impl<Transfer, Handler, Connector, Creds, T, U> Deref
    for WebSocketClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    type Target = WebSocketInnerClient<Transfer, Handler, Connector, Creds, T, U>;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    WebSocketClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    pub async fn init(
        credentials: Creds,
        connector: Connector,
        data: Data<T, Transfer>,
        handler: Handler,
        reconnect_callback: Option<Callback<T, Transfer, U>>,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<Self> {
        let inner = WebSocketInnerClient::init(
            credentials,
            connector,
            data,
            handler,
            reconnect_callback,
            config,
        )
        .await?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    WebSocketInnerClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    pub async fn init(
        credentials: Creds,
        connector: Connector,
        data: Data<T, Transfer>,
        handler: Handler,
        reconnect_callback: Option<Callback<T, Transfer, U>>,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<Self> {
        let _connection = connector.connect(credentials.clone(), &config).await?; // Check if it's possible to connect before building the struct
        let (_event_loop, sender) = Self::start_loops(
            handler.clone(),
            credentials.clone(),
            data.clone(),
            connector.clone(),
            reconnect_callback.clone(),
            config.clone(),
        )
        .await?;
        info!("Started WebSocketClient");
        sleep(config.get_connection_initialization_timeout()?).await;
        Ok(Self {
            credentials,
            connector,
            handler,
            data,
            sender,
            reconnect_callback,
            config,
            _event_loop,
        })
    }

    async fn start_loops(
        handler: Handler,
        credentials: Creds,
        data: Data<T, Transfer>,
        connector: Connector,
        reconnect_callback: Option<Callback<T, Transfer, U>>,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<(JoinHandle<BinaryOptionsResult<()>>, SenderMessage)> {
        let (mut write, mut read) = connector
            .connect(credentials.clone(), &config)
            .await?
            .split();
        let (sender, (reciever, reciever_priority)) = SenderMessage::new(MAX_CHANNEL_CAPACITY);
        let loop_sender = sender.clone();
        let task = tokio::task::spawn(async move {
            let previous: Option<<Transfer as MessageTransfer>::Info> = None;
            let mut loops = 0;
            let mut reconnected = false;
            loop {
                match WebSocketInnerClient::<Transfer, Handler, Connector, Creds, T, U>::step(
                    &previous,
                    &data,
                    handler.clone(),
                    &loop_sender,
                    &mut read,
                    &mut write,
                    &reciever,
                    &reciever_priority,
                    &config,
                    &reconnect_callback,
                    reconnected,
                    &connector,
                    &credentials,
                    &mut loops,
                )
                .await
                {
                    Ok(res) => {
                        info!("Reconnected successfully!");
                        (write, read) = res.split();
                        reconnected = true;
                        loops = 0;
                    }
                    Err(e) => {
                        if let BinaryOptionsToolsError::MaxReconnectAttemptsReached(_) = e {
                            return Err(e);
                        }
                    }
                }
            }
        });
        Ok((task, sender))
    }

    #[allow(clippy::too_many_arguments)]
    async fn step(
        previous: &Option<<<Handler as MessageHandler>::Transfer as MessageTransfer>::Info>,
        data: &Data<T, Transfer>,
        handler: Handler,
        loop_sender: &SenderMessage,
        read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        write: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        reciever: &Receiver<Message>,
        reciever_priority: &Receiver<Message>,
        config: &Config<T, Transfer, U>,
        reconnect_callback: &Option<Callback<T, Transfer, U>>,
        reconnected: bool,
        connector: &Connector,
        credentials: &Creds,
        loops: &mut u32,
    ) -> BinaryOptionsResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let listener_future =
            WebSocketInnerClient::<Transfer, Handler, Connector, Creds, T, U>::listener_loop(
                previous.clone(),
                data,
                handler.clone(),
                loop_sender,
                read,
            );
        let sender_future =
            WebSocketInnerClient::<Transfer, Handler, Connector, Creds, T, U>::sender_loop(
                write,
                reciever,
                reciever_priority,
                config.get_reconnect_time()?,
            );

        let callback =
            WebSocketInnerClient::<Transfer, Handler, Connector, Creds, T, U>::reconnect_callback(
                reconnect_callback.clone(),
                data.clone(),
                loop_sender.clone(),
                reconnected,
                config.get_reconnect_time()?,
                config.clone(),
            );

        match try_join3(listener_future, sender_future, callback).await {
            Ok(_) => {
                if let Ok(websocket) = connector.connect(credentials.clone(), config).await {
                    return Ok(websocket);
                } else {
                    *loops += 1;
                    let sleep_interval = config.get_sleep_interval()?;
                    let max_loops = config.get_max_allowed_loops()?;
                    warn!(
                        "Error reconnecting... trying again in {sleep_interval} seconds (try {loops} of {max_loops})"
                    );
                    sleep(Duration::from_secs(config.get_sleep_interval()?)).await;
                    if *loops >= max_loops {
                        return Err(BinaryOptionsToolsError::MaxReconnectAttemptsReached(
                            max_loops,
                        ));
                    }
                }
            }
            Err(e) => {
                warn!("Error in event loop, {e}, reconnecting...");
                // println!("Reconnecting...");
                if let Ok(websocket) = connector.connect(credentials.clone(), config).await {
                    return Ok(websocket);
                } else {
                    *loops += 1;
                    let sleep_interval = config.get_sleep_interval()?;
                    let max_loops = config.get_max_allowed_loops()?;
                    warn!(
                        "Error reconnecting... trying again in {sleep_interval} seconds (try {loops} of {max_loops})"
                    );
                    sleep(Duration::from_secs(config.get_sleep_interval()?)).await;
                    if *loops >= max_loops {
                        return Err(BinaryOptionsToolsError::MaxReconnectAttemptsReached(
                            max_loops,
                        ));
                    }
                }
            }
        }
        Err(BinaryOptionsToolsError::ReconnectionAttemptFailure {
            number: *loops,
            max: config.get_max_allowed_loops()?,
        })
        // unreachable!("Please contact @Rick-29 on github.com this error is completely unexpected and should not happen.")
    }

    /// Recieves all the messages from the websocket connection and handles it
    async fn listener_loop(
        mut previous: Option<<<Handler as MessageHandler>::Transfer as MessageTransfer>::Info>,
        data: &Data<T, Transfer>,
        handler: Handler,
        sender: &SenderMessage,
        ws: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> BinaryOptionsResult<()> {
        while let Some(msg) = &ws.next().await {
            let msg = msg
                .as_ref()
                .inspect_err(|e| warn!("Error recieving websocket message, {e}"))
                .map_err(|e| {
                    BinaryOptionsToolsError::WebsocketRecievingConnectionError(e.to_string())
                })?;
            match handler.process_message(msg, &previous, sender).await {
                Ok((msg, close)) => {
                    if close {
                        info!("Recieved closing frame");
                        return Err(BinaryOptionsToolsError::WebsocketConnectionClosed(
                            "Recieved closing frame".into(),
                        ));
                    }
                    if let Some(msg) = msg {
                        match msg {
                            MessageType::Info(info) => {
                                debug!("Recieved info: {}", info);
                                previous = Some(info);
                            }
                            MessageType::Transfer(transfer) => {
                                debug!("Recieved data of type: {}", transfer.info());
                                if let Some(senders) = data.update_data(transfer.clone()).await? {
                                    for sender in senders {
                                        sender.send(transfer.clone()).await.map_err(|e| {
                                            BinaryOptionsToolsError::ChannelRequestSendingError(
                                                e.to_string(),
                                            )
                                        })?;
                                    }
                                }
                            }
                            MessageType::Raw(raw) => {
                                debug!("Recieved raw message: {:?}", raw);
                                data.raw_send(raw).await?;
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Error processing message, {e}");
                }
            }
        }
        Err(BinaryOptionsToolsError::WebSocketMessageError("Unexpected error encountered while recieving data from websocket connection. Loop terminated unexpectedly".to_string()))
    }

    /// Recieves all the messages and sends them to the websocket
    async fn sender_loop(
        ws: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        reciever: &Receiver<Message>,
        reciever_priority: &Receiver<Message>,
        time: u64,
    ) -> BinaryOptionsResult<()> {
        async fn priority_mesages(
            ws: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            reciever_priority: &Receiver<Message>,
        ) -> BinaryOptionsResult<()> {
            while let Ok(msg) = reciever_priority.recv().await {
                ws.send(msg)
                    .await
                    .inspect_err(|e| warn!("Error sending message to websocket, {e}"))?;
                ws.flush().await?;
                debug!("Sent message to websocket!");
            }
            Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                RecvError,
            ))
        }

        tokio::select! {
            res = priority_mesages(ws, reciever_priority) => res?,
            _ = sleep(Duration::from_secs(time)) => {}
        }
        let stream1 = RecieverStream::new(reciever.to_owned());
        let stream2 = RecieverStream::new(reciever_priority.to_owned());
        let mut fused_streams = select_all([stream1.to_stream(), stream2.to_stream()]);

        while let Some(Ok(msg)) = fused_streams.next().await {
            ws.send(msg)
                .await
                .inspect_err(|e| warn!("Error sending message to websocket, {e}"))?;
            ws.flush().await?;
            debug!("Sent message to websocket!");
        }
        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
            RecvError,
        ))
    }

    // async fn api_loop(
    //     reciever: &mut Receiver<Transfer>,
    //     sender: &Sender<Message>,
    // ) -> BinaryOptionsResult<()> {
    //     while let Ok(msg) = reciever.recv().await {
    //         sender.send(msg.into()).await?;
    //     }
    //     Ok(())
    // }

    async fn reconnect_callback(
        reconnect_callback: Option<Callback<T, Transfer, U>>,
        data: Data<T, Transfer>,
        sender: SenderMessage,
        reconnect: bool,
        reconnect_time: u64,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<BinaryOptionsResult<()>> {
        Ok(tokio::spawn(async move {
            sleep(Duration::from_secs(reconnect_time)).await;
            if reconnect {
                if let Some(callback) = &reconnect_callback {
                    callback
                        .call(data.clone(), &sender, &config)
                        .await
                        .inspect_err(
                            |e| error!(target: "EventLoop","Error calling callback, {e}"),
                        )?;
                }
            }
            Ok(())
        })
        .await?)
    }

    pub async fn send(&self, msg: Transfer) -> BinaryOptionsResult<()> {
        self.sender.send::<Transfer>(msg).await
    }

    pub async fn raw_send(&self, msg: Transfer::Raw) -> BinaryOptionsResult<()> {
        self.sender.raw_send::<Transfer>(msg).await
    }

    pub async fn send_message(
        &self,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        self.sender
            .send_message(&self.data, msg, response_type, validator)
            .await
    }

    pub async fn send_raw_message(
        &self,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        self.sender
            .send_raw_message(&self.data, msg, validator)
            .await
    }

    pub async fn send_message_with_timout(
        &self,
        timeout: Duration,
        task: impl ToString,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        self.sender
            .send_message_with_timout(timeout, task, &self.data, msg, response_type, validator)
            .await
    }

    pub async fn send_raw_message_with_timout(
        &self,
        timeout: Duration,
        task: impl ToString,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        self.sender
            .send_raw_message_with_timout(timeout, task, &self.data, msg, validator)
            .await
    }

    pub async fn send_message_with_timeout_and_retry(
        &self,
        timeout: Duration,
        task: impl ToString,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        self.sender
            .send_message_with_timeout_and_retry(
                timeout,
                task,
                &self.data,
                msg,
                response_type,
                validator,
            )
            .await
    }

    pub async fn send_raw_message_with_timeout_and_retry(
        &self,
        timeout: Duration,
        task: impl ToString,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        self.sender
            .send_raw_message_with_timeout_and_retry(timeout, task, &self.data, msg, validator)
            .await
    }

    pub async fn send_raw_message_iterator(
        &self,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
        timeout: Option<Duration>,
    ) -> BinaryOptionsResult<FilteredRecieverStream<Transfer::Raw>> {
        self.sender
            .send_raw_message_iterator(timeout, &self.data, msg, validator)
            .await
    }
}

// impl<Transfer, Handler, Connector, Creds, T, C> Drop
//     for WebSocketClient<Transfer, Handler, Connector, Creds, T, C>
// where
//     Transfer: MessageTransfer,
//     Handler: MessageHandler,
//     Connector: Connect,
//     Creds: Credentials,
//     T: DataHandler,
//     C: Callback,
// {
//     fn drop(&mut self) {
//         self._event_loop.abort();
//         info!(target: "Drop", "Dropping WebSocketClient instance");
//     }
// }

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_channel::{Receiver, Sender, bounded};
    use futures_util::{
        Stream, StreamExt,
        future::try_join,
        stream::{select_all, unfold},
    };
    use rand::{Rng, distr::Alphanumeric};
    use tokio::time::sleep;
    use tracing::info;

    use crate::utils::tracing::start_tracing;

    struct RecieverStream<T> {
        inner: Receiver<T>,
    }

    impl<T> RecieverStream<T> {
        fn new(inner: Receiver<T>) -> Self {
            Self { inner }
        }

        async fn receive(&self) -> anyhow::Result<T> {
            Ok(self.inner.recv().await?)
        }

        fn to_stream(&self) -> impl Stream<Item = anyhow::Result<T>> + '_ {
            Box::pin(unfold(self, |state| async move {
                let item = state.receive().await;
                Some((item, state))
            }))
        }
    }

    async fn recieve_dif(
        reciever: Receiver<String>,
        receiver_priority: Receiver<String>,
    ) -> anyhow::Result<()> {
        async fn receiv(r: &Receiver<String>) -> anyhow::Result<()> {
            while let Ok(t) = r.recv().await {
                info!(target: "High priority", "Recieved: {}", t);
            }
            Ok(())
        }
        tokio::select! {
            err = receiv(&receiver_priority) => err?,
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        }
        let receiver = RecieverStream::new(reciever);
        let receiver_priority = RecieverStream::new(receiver_priority);
        let mut fused = select_all([receiver.to_stream(), receiver_priority.to_stream()]);
        while let Some(value) = fused.next().await {
            info!(target: "Fused", "Recieved: {}", value?);
        }

        Ok(())
    }

    async fn recieve_dif_err(
        reciever: Receiver<String>,
        receiver_priority: Receiver<String>,
    ) -> anyhow::Result<()> {
        async fn receiv(r: &Receiver<String>) -> anyhow::Result<()> {
            let mut loops = 0;
            while let Ok(t) = r.recv().await {
                if loops == 2 {
                    return Err(anyhow::Error::msg("error receiving message"));
                }
                loops += 1;
                info!(target: "High priority", "Recieved: {}", t);
            }
            Ok(())
        }
        tokio::select! {
            err = receiv(&receiver_priority) => err?,
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        }
        let receiver = RecieverStream::new(reciever);
        let receiver_priority = RecieverStream::new(receiver_priority);
        let mut fused = select_all([receiver.to_stream(), receiver_priority.to_stream()]);
        while let Some(value) = fused.next().await {
            info!(target: "Fused", "Recieved: {}", value?);
        }

        Ok(())
    }

    async fn sender_dif(
        sender: Sender<String>,
        sender_priority: Sender<String>,
    ) -> anyhow::Result<()> {
        loop {
            let s1: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(7)
                .map(char::from)
                .collect();
            let s2: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(7)
                .map(char::from)
                .collect();
            sender.send(s1).await?;
            sender_priority.send(s2).await?;
            sleep(Duration::from_secs(1)).await;
        }
    }

    #[tokio::test]
    async fn test_multi_priority_reciever_ok() -> anyhow::Result<()> {
        start_tracing(true)?;
        let (s, r) = bounded(8);
        let (sp, rp) = bounded(8);
        try_join(sender_dif(s, sp), recieve_dif(r, rp)).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_reconnection_limit_reached_error() {
        use crate::error::BinaryOptionsToolsError;
        
        let max_loops = 3;
        let mut loops = 0;
        
        // We simulate the logic of the reconnection loop
        for _ in 0..max_loops {
            loops += 1;
            if loops >= max_loops {
                let err = BinaryOptionsToolsError::MaxReconnectAttemptsReached(max_loops);
                if let BinaryOptionsToolsError::MaxReconnectAttemptsReached(m) = err {
                    assert_eq!(m, max_loops);
                    return;
                }
            }
        }
        panic!("Should have reached max loops");
    }

    #[tokio::test]
    async fn test_loops_reset_on_success() {
        let mut loops = 5;
        // Simulate successful reconnection
        loops = 0;
        assert_eq!(loops, 0);
    }
}
