use std::{sync::Arc, time::Duration};

use kanal::{AsyncReceiver, ReceiveError};
use futures_util::{Stream, stream::unfold};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    error::{CoreError, CoreResult}, traits::Rule, utils::time::timeout
};


pub struct RecieverStream {
    inner: AsyncReceiver<Message>,
    timeout: Option<Duration>,
}

pub struct FilteredRecieverStream {
    inner: AsyncReceiver<Message>,
    timeout: Option<Duration>,
    filter: Box<dyn Rule + Send + Sync>,
}

impl RecieverStream {
    pub fn new(inner: AsyncReceiver<Message>) -> Self {
        Self {
            inner,
            timeout: None,
        }
    }

    pub fn new_timed(inner: AsyncReceiver<Message>, timeout: Option<Duration>) -> Self {
        Self { inner, timeout }
    }

    async fn receive(&self) -> CoreResult<Message> {
        match self.timeout {
            Some(time) => timeout(time, self.inner.recv(), "RecieverStream".to_string()).await,
            None => Ok(self.inner.recv().await?),
        }
    }

    pub fn to_stream(&self) -> impl Stream<Item = CoreResult<Message>> + '_ {
        Box::pin(unfold(self, move |state| async move {
            let item = state.receive().await;
            Some((item, state))
        }))
    }

    pub fn to_stream_static(self: Arc<Self>) -> impl Stream<Item = CoreResult<Message>> + 'static
    {
        Box::pin(unfold(self, async |state| {
            let item = state.receive().await;
            Some((item, state))
        }))
    }
}

impl FilteredRecieverStream {
    pub fn new(
        inner: AsyncReceiver<Message>,
        timeout: Option<Duration>,
        filter: Box<dyn Rule + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            timeout,
            filter,
        }
    }

    pub fn new_base(inner: AsyncReceiver<Message>) -> Self {
        Self::new(inner, None, default_filter())
    }

    pub fn new_filtered(
        inner: AsyncReceiver<Message>,
        filter: Box<dyn Rule + Send + Sync>,
    ) -> Self {
        Self::new(inner, None, filter)
    }

    async fn recv(&self) -> CoreResult<Message> {
        while let Ok(msg) = self.inner.recv().await {
            if self.filter.call(&msg) {
                return Ok(msg);
            }
        }
        Err(CoreError::ChannelReceiver(ReceiveError::Closed))
    }

    async fn receive(&self) -> CoreResult<Message> {
        match self.timeout {
            Some(time) => timeout(time, self.recv(), "RecieverStream".to_string()).await,
            None => Ok(self.inner.recv().await?),
        }
    }

    pub fn to_stream(&self) -> impl Stream<Item = CoreResult<Message>> + '_ {
        Box::pin(unfold(self, move |state| async move {
            let item = state.receive().await;
            Some((item, state))
        }))
    }

    pub fn to_stream_static(self: Arc<Self>) -> impl Stream<Item = CoreResult<Message>> + 'static
    {
        Box::pin(unfold(self, async |state| {
            let item = state.receive().await;
            Some((item, state))
        }))
    }
}

fn default_filter() -> Box<dyn Rule + Send + Sync> {
    Box::new(move |_: &Message| true)
}
