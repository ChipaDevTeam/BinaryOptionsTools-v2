use std::{sync::Arc, time::Duration};

use async_channel::{Receiver, RecvError};
use futures_util::{stream::unfold, Stream};

use crate::{
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    utils::time::timeout,
};

use super::traits::ValidatorTrait;

pub struct RecieverStream<T> {
    inner: Receiver<T>,
    timeout: Option<Duration>,
}

pub struct FilteredRecieverStream<T> {
    inner: Receiver<T>,
    timeout: Option<Duration>,
    filter: Box<dyn ValidatorTrait<T> + Send + Sync>,
}

impl<T> RecieverStream<T> {
    pub fn new(inner: Receiver<T>) -> Self {
        Self {
            inner,
            timeout: None,
        }
    }

    pub fn new_timed(inner: Receiver<T>, timeout: Option<Duration>) -> Self {
        Self { inner, timeout }
    }

    async fn receive(&self) -> BinaryOptionsResult<T> {
        match self.timeout {
            Some(time) => timeout(time, self.inner.recv(), "RecieverStream".to_string()).await,
            None => Ok(self.inner.recv().await?),
        }
    }

    pub fn to_stream(&self) -> impl Stream<Item = BinaryOptionsResult<T>> + '_ {
        Box::pin(unfold(self, move |state| async move {
            let item = state.receive().await;
            Some((item, state))
        }))
    }

    pub fn to_stream_static(self: Arc<Self>) -> impl Stream<Item = BinaryOptionsResult<T>> + 'static
    where
        T: 'static,
    {
        Box::pin(unfold(self, async |state| {
            let item = state.receive().await;
            Some((item, state))
        }))
    }
}

impl<T> FilteredRecieverStream<T> {
    pub fn new(
        inner: Receiver<T>,
        timeout: Option<Duration>,
        filter: Box<dyn ValidatorTrait<T> + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            timeout,
            filter,
        }
    }

    pub fn new_base(inner: Receiver<T>) -> Self {
        Self::new(inner, None, default_filter())
    }

    pub fn new_filtered(
        inner: Receiver<T>,
        filter: Box<dyn ValidatorTrait<T> + Send + Sync>,
    ) -> Self {
        Self::new(inner, None, filter)
    }

    async fn recv(&self) -> BinaryOptionsResult<T> {
        while let Ok(msg) = self.inner.recv().await {
            if self.filter.validate(&msg) {
                return Ok(msg);
            }
        }
        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
            RecvError,
        ))
    }

    async fn receive(&self) -> BinaryOptionsResult<T> {
        match self.timeout {
            Some(time) => timeout(time, self.recv(), "RecieverStream".to_string()).await,
            None => Ok(self.inner.recv().await?),
        }
    }

    pub fn to_stream(&self) -> impl Stream<Item = BinaryOptionsResult<T>> + '_ {
        Box::pin(unfold(self, move |state| async move {
            let item = state.receive().await;
            Some((item, state))
        }))
    }

    pub fn to_stream_static(self: Arc<Self>) -> impl Stream<Item = BinaryOptionsResult<T>> + 'static
    where
        T: 'static,
    {
        Box::pin(unfold(self, async |state| {
            let item = state.receive().await;
            Some((item, state))
        }))
    }
}

fn default_filter<T>() -> Box<dyn ValidatorTrait<T> + Send + Sync> {
    Box::new(move |_: &T| true)
}
