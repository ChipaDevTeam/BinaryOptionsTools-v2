use crate::error::BinaryErrorPy;
use crate::pocketoption::RawPocketOption;
use async_trait::async_trait;
use binary_options_tools::framework::market::Market;
use binary_options_tools::framework::virtual_market::VirtualMarket;
use binary_options_tools::framework::{Bot, Context, Strategy};
use binary_options_tools::pocketoption::candle::Candle;
use binary_options_tools::pocketoption::error::PocketResult;
use binary_options_tools::utils::f64_to_decimal;
use pyo3::prelude::*;
use rust_decimal::prelude::ToPrimitive;
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
        let inner = Python::attach(|py| self.inner.clone_ref(py));
        let client = ctx.client.clone();
        let market = ctx.market.clone();

        tokio::task::spawn_blocking(move || -> PocketResult<()> {
            Python::attach(|py| {
                let py_ctx = PyContext {
                    client: Some(client),
                    market,
                };
                inner.call_method1(py, "on_start", (py_ctx,)).map_err(|e| {
                    binary_options_tools::pocketoption::error::PocketError::General(format!(
                        "Python on_start error: {}",
                        e
                    ))
                })
            })
            .map(|_| ())
        })
        .await
        .map_err(|e| {
            binary_options_tools::pocketoption::error::PocketError::General(format!(
                "Spawn blocking error: {}",
                e
            ))
        })??;
        Ok(())
    }

    async fn on_candle(&self, ctx: &Context, asset: &str, candle: &Candle) -> PocketResult<()> {
        let candle_json = serde_json::to_string(candle).map_err(|e| {
            binary_options_tools::pocketoption::error::PocketError::General(e.to_string())
        })?;
        let asset = asset.to_string();
        let inner = Python::attach(|py| self.inner.clone_ref(py));
        let client = ctx.client.clone();
        let market = ctx.market.clone();

        tokio::task::spawn_blocking(move || -> PocketResult<()> {
            Python::attach(|py| {
                let py_ctx = PyContext {
                    client: Some(client),
                    market,
                };
                inner
                    .call_method1(py, "on_candle", (py_ctx, asset, candle_json))
                    .map_err(|e| {
                        binary_options_tools::pocketoption::error::PocketError::General(format!(
                            "Python on_candle error: {}",
                            e
                        ))
                    })
            })
            .map(|_| ())
        })
        .await
        .map_err(|e| {
            binary_options_tools::pocketoption::error::PocketError::General(format!(
                "Spawn blocking error: {}",
                e
            ))
        })??;
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
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid amount: {}", amount)))?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let res = market
                .buy(&asset, decimal_amount, time)
                .await
                .map_err(BinaryErrorPy::from)?;
            let deal = serde_json::to_string(&res.1).map_err(BinaryErrorPy::from)?;
            let result = vec![res.0.to_string(), deal];
            Ok(result)
        })
    }

    pub fn balance<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let market = self.market.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(market.balance().await.to_f64().unwrap_or_default())
        })
    }
}

#[pyclass]
pub struct PyVirtualMarket {
    pub(crate) inner: Arc<VirtualMarket>,
}

#[pymethods]
impl PyVirtualMarket {
    #[new]
    pub fn new(initial_balance: f64) -> PyResult<Self> {
        let decimal_balance = f64_to_decimal(initial_balance).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid initial balance: {}",
                initial_balance
            ))
        })?;
        Ok(Self {
            inner: Arc::new(VirtualMarket::new(decimal_balance)),
        })
    }

    pub fn update_price<'py>(
        &self,
        py: Python<'py>,
        asset: String,
        price: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let decimal_price = f64_to_decimal(price)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid price: {}", price)))?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.update_price(&asset, decimal_price).await;
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
            let subscription =
                binary_options_tools::pocketoption::candle::SubscriptionType::time_aligned(
                    std::time::Duration::from_secs(period as u64),
                )
                .map_err(BinaryErrorPy::from)?;

            bot.add_asset(asset, subscription);
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Bot already consumed or run() called",
            ))
        }
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
