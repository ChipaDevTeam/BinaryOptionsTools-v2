use crate::error::BinaryErrorPy;
use crate::pocketoption::RawPocketOption;
use async_trait::async_trait;
use binary_options_tools::framework::market::Market;
use binary_options_tools::framework::virtual_market::VirtualMarket;
use binary_options_tools::framework::{Bot, Context, Strategy};
use binary_options_tools::pocketoption::candle::Candle;
use binary_options_tools::pocketoption::error::PocketResult;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::sync::Arc;

#[pyclass(subclass)]
pub struct PyStrategy {}

#[pymethods]
impl PyStrategy {
    #[new]
    pub fn new() -> Self {
        Self {}
    }

    pub fn on_start(&self, _ctx: PyContext) -> PyResult<()> {
        Ok(())
    }

    pub fn on_candle(&self, _ctx: PyContext, _asset: String, _candle_json: String) -> PyResult<()> {
        Ok(())
    }
}

pub struct StrategyWrapper {
    pub inner: Py<PyStrategy>,
}

#[async_trait]
impl Strategy for StrategyWrapper {
    async fn on_start(&self, ctx: &Context) -> PocketResult<()> {
        Python::attach(|py| {
            let py_ctx = PyContext {
                client: Some(ctx.client.clone()),
                market: ctx.market.clone(),
            };
            self.inner
                .call_method1(py, "on_start", (py_ctx,))
                .map_err(|e| {
                    binary_options_tools::pocketoption::error::PocketError::General(format!("Python on_start error: {}", e))
                })
        }).map(|_| ())?;
        Ok(())
    }

    async fn on_candle(&self, ctx: &Context, asset: &str, candle: &Candle) -> PocketResult<()> {
        let candle_json = serde_json::to_string(candle).unwrap_or_default();
        let asset = asset.to_string();
        Python::attach(|py| {
            let py_ctx = PyContext {
                client: Some(ctx.client.clone()),
                market: ctx.market.clone(),
            };
            self.inner
                .call_method1(py, "on_candle", (py_ctx, asset, candle_json))
                .map_err(|e| {
                    binary_options_tools::pocketoption::error::PocketError::General(format!("Python on_candle error: {}", e))
                })
        }).map(|_| ())?;
        Ok(())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyContext {
    pub client: Option<Arc<binary_options_tools::pocketoption::pocket_client::PocketOption>>,
    pub market: Arc<dyn Market>,
}

#[pymethods]
impl PyContext {
    pub fn buy<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        amount: f64,
        time: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let market = self.market.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let res = market
                .buy(&asset, amount, time)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Ok(result)
        })
    }

    pub fn balance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let market = self.market.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(market.balance().await) })
    }
}

#[pyclass]
pub struct PyVirtualMarket {
    pub(crate) inner: Arc<VirtualMarket>,
}

#[pymethods]
impl PyVirtualMarket {
    #[new]
    pub fn new(initial_balance: f64) -> Self {
        Self {
            inner: Arc::new(VirtualMarket::new(initial_balance)),
        }
    }

    pub fn update_price<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        price: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.update_price(&asset, price).await;
            Ok(())
        })
    }
}

#[pyclass]
pub struct PyBot {
    inner: Option<Bot>,
}

#[pymethods]
impl PyBot {
    #[new]
    #[pyo3(signature = (client, strategy, virtual_market=None))]
    pub fn new(
        client: RawPocketOption,
        strategy: Py<PyStrategy>,
        virtual_market: Option<Bound<'_, PyVirtualMarket>>,
    ) -> Self {
        let wrapper = StrategyWrapper { inner: strategy };
        let mut bot = Bot::new(client.client.clone(), Box::new(wrapper));
        if let Some(vm) = virtual_market {
            bot = bot.with_market(vm.borrow().inner.clone());
        }
        Self { inner: Some(bot) }
    }

    pub fn add_asset(&mut self, asset: String, period: u32) -> PyResult<()> {
        if let Some(bot) = &mut self.inner {
            let subscription = binary_options_tools::pocketoption::candle::SubscriptionType::time_aligned(
                std::time::Duration::from_secs(period as u64),
            )
            .map_err(BinaryErrorPy::from)?;
            
            bot.add_asset(asset, subscription);
        }
        Ok(())
    }

    pub fn run<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let bot = self.inner.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Bot already running or consumed")
        })?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            bot.run().await.map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }
}
