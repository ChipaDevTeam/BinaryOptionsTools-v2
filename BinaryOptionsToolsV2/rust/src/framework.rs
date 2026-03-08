use crate::error::BinaryErrorPy;
use crate::pocketoption::RawPocketOption;
use crate::runtime::get_runtime;

use binary_options_tools::framework::market::Market;
use binary_options_tools::framework::virtual_market::VirtualMarket;
use binary_options_tools::framework::{Bot, Context, Strategy};
use binary_options_tools::pocketoption::candle::Candle;
use binary_options_tools::pocketoption::error::{PocketError, PocketResult};
use binary_options_tools::utils::f64_to_decimal;

use pyo3::prelude::*;

use async_trait::async_trait;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub enum Action {
    Call,
    Put,
}

#[pyclass(subclass)]
pub struct PyStrategy {
    indicators: HashMap<String, Py<PyAny>>,
    #[pyo3(get)]
    pub current_candle: u32,
}

#[pymethods]
impl PyStrategy {
    #[new]
    pub fn new() -> Self {
        Self {
            indicators: HashMap::new(),
            current_candle: 0,
        }
    }

    pub fn on_start(&self, _ctx: PyContext) -> PyResult<()> {
        Ok(())
    }

    pub fn on_candle(&self, _ctx: PyContext, _asset: String, _candle_json: String) -> PyResult<()> {
        Ok(())
    }

    pub fn on_balance(&self, _ctx: PyContext, _balance: f64) -> PyResult<()> {
        Ok(())
    }

    pub fn trade<'py>(
        &self,
        py: Python<'py>,
        ctx: PyContext,
        asset: String,
        amount: f64,
        timeframe: u32,
        direction: Action,
    ) -> PyResult<Vec<String>> {
        let market = ctx.market.clone();
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| BinaryErrorPy::NotAllowed(format!("Invalid amount: {}", amount)))?;
        let trade_future = async move {
            let (id, deal) = match direction {
                Action::Call => market
                    .buy(&asset, decimal_amount, timeframe)
                    .await
                    .map_err(BinaryErrorPy::from),

                Action::Put => market
                    .sell(&asset, decimal_amount, timeframe)
                    .await
                    .map_err(BinaryErrorPy::from),
            }?;
            let trades = Vec::from([
                id.to_string(),
                serde_json::to_string(&deal).map_err(BinaryErrorPy::from)?,
            ]);
            Result::<Vec<String>, BinaryErrorPy>::Ok(trades)
        };

        Ok(get_runtime(py)?.block_on(trade_future)?)
    }

    pub fn result<'py>(&self, py: Python<'py>, ctx: PyContext, id: String) -> PyResult<String> {
        let market = ctx.market.clone();
        let uuid = Uuid::parse_str(&id)
            .map_err(|e| BinaryErrorPy::NotAllowed(format!("Invalid UUID: {}", e)))?;
        let future = async move {
            let res = market.result(uuid).await.map_err(BinaryErrorPy::from)?;
            serde_json::to_string(&res).map_err(BinaryErrorPy::from)
        };
        Ok(get_runtime(py)?.block_on(future)?)
    }

    pub fn add(&mut self, name: String, indicator: Py<PyAny>) -> PyResult<()> {
        self.indicators.insert(name.clone(), indicator);
        info!(target: "PyStrategy", "Added indicator '{}' to strategy", name);
        Ok(())
    }

    pub fn get(&self, name: String) -> PyResult<Option<&Py<PyAny>>> {
        Ok(self.indicators.get(&name))
    }

    pub fn list_indicators(&self) -> PyResult<Vec<(String, String)>> {
        self.indicators
            .iter()
            .map(|(name, indicator)| {
                let indicator_str = Python::attach(|py| {
                    indicator.call_method0(py, "__str__")?.extract::<String>(py)
                })?;
                Ok((name.clone(), indicator_str))
            })
            .collect()
    }

    pub fn update<'py>(&mut self, candle: String) -> PyResult<()> {
        self.current_candle += 1;
        for indicator in self.indicators.values() {
            Python::attach(|py| {
                indicator
                    .call_method1(py, "update", (candle.clone(),))
                    .map_err(|e| {
                        BinaryErrorPy::NotAllowed(format!("Failed to update indicator: {}", e))
                    })
            })?;
        }
        info!(target: "PyStrategy", "Updated indicators with new candle: {}", self.current_candle);
        Ok(())
    }

    pub fn reset(&mut self) -> PyResult<()> {
        for indicator in self.indicators.values() {
            Python::attach(|py| {
                indicator.call_method0(py, "reset").map_err(|e| {
                    BinaryErrorPy::NotAllowed(format!("Failed to reset indicator: {}", e))
                })
            })?;
        }
        self.current_candle = 0;
        Ok(())
    }

    pub fn period(&self) -> PyResult<u32> {
        let mut max_period = 0;
        for indicator in self.indicators.values() {
            let period: u32 = Python::attach(|py| {
                indicator
                    .call_method0(py, "period")
                    .map_err(|e| {
                        BinaryErrorPy::NotAllowed(format!(
                            "Failed to get period from indicator: {}",
                            e
                        ))
                    })?
                    .extract(py)
                    .map_err(|e| {
                        BinaryErrorPy::NotAllowed(format!("Failed to extract period as u32: {}", e))
                    })
            })?;
            if period > max_period {
                max_period = period;
            }
        }
        Ok(max_period)
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
                inner
                    .call_method1(py, "on_start", (py_ctx,))
                    .map_err(|e| PocketError::General(format!("Python on_start error: {}", e)))
            })
            .map(|_| ())
        })
        .await
        .map_err(|e| PocketError::General(format!("Spawn blocking error: {}", e)))??;
        Ok(())
    }

    async fn on_candle(&self, ctx: &Context, asset: &str, candle: &Candle) -> PocketResult<()> {
        let candle_json =
            serde_json::to_string(candle).map_err(|e| PocketError::General(e.to_string()))?;
        let asset = asset.to_string();
        let inner = Python::attach(|py| self.inner.clone_ref(py));
        let client = ctx.client.clone();
        let market = ctx.market.clone();
        let period = Python::attach(|py| {
            inner
                .call_method0(py, "period")
                .map_err(|e| PocketError::General(format!("Python period error: {}", e)))
                .map(|obj| obj.extract::<u32>(py))
        })?
        .map_err(|e| PocketError::General(format!("Python period extract error: {}", e)))?;
        let current_candle = Python::attach(|py| {
            inner
                .getattr(py, "current_candle")
                .map_err(|e| PocketError::General(format!("Python current_candle error: {}", e)))
                .and_then(|obj| {
                    obj.extract::<u32>(py).map_err(|e| {
                        PocketError::General(format!("Python current_candle extract error: {}", e))
                    })
                })
        })?;

        if current_candle < period {
            Python::attach(|py| {
                inner
                    .call_method1(py, "update", (candle_json.clone(),))
                    .map_err(|e| PocketError::General(format!("Python update error: {}", e)))
            })?;
            info!(target: "StrategyWrapper", "Loading period: candle {} of {}", current_candle +1, period);
            return Ok(());
        }

        tokio::task::spawn_blocking(move || -> PocketResult<()> {
            Python::attach(|py| {
                let py_ctx = PyContext {
                    client: Some(client),
                    market,
                };
                inner
                    .call_method1(py, "on_candle", (py_ctx, asset, candle_json))
                    .map_err(|e| PocketError::General(format!("Python on_candle error: {}", e)))
            })
            .map(|_| ())
        })
        .await
        .map_err(|e| PocketError::General(format!("Spawn blocking error: {}", e)))??;

        Ok(())
    }

    async fn on_balance_update(&self, ctx: &Context, balance: Decimal) -> PocketResult<()> {
        let balance = balance.to_f64().unwrap_or(-1.0);
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
                    .call_method1(py, "on_balance", (py_ctx, balance))
                    .map_err(|e| PocketError::General(format!("Python on_balance error: {}", e)))
            })
            .map(|_| ())
        })
        .await
        .map_err(|e| PocketError::General(format!("Spawn blocking error: {}", e)))??;

        Ok(())
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyContext {
    pub client: Option<Arc<binary_options_tools::pocketoption::pocket_client::PocketOption>>,
    pub market: Arc<dyn Market>,
}

#[pymethods]
impl PyContext {
    /// Places a buy (Call) order asynchronously.
    ///
    /// :param asset: The asset to trade (e.g. "EURUSD_otc").
    /// :param amount: The amount to trade.
    /// :param time: The duration of the trade in seconds.
    /// :return: A list [trade_id, deal_json].
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

    /// Fetches the current balance asynchronously.
    ///
    /// :return: The current balance as a float.
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

    /// Updates the price of an asset in the virtual market.
    /// This is an asynchronous method.
    ///
    /// :param asset: The asset identifier.
    /// :param price: The new price.
    /// :return: None
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

    pub fn with_update_interval(&mut self, millis: u64) -> PyResult<()> {
        if let Some(bot) = &mut self.inner {
            bot.with_update_interval(Duration::from_millis(millis));
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Bot already consumed or run() called",
            ))
        }
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

    /// Runs the bot's execution loop.
    /// This is an asynchronous method that will block the current task until the bot is stopped.
    ///
    /// :return: None
    pub fn run<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut bot = self.inner.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Bot already running or consumed")
        })?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            bot.run().await.map_err(BinaryErrorPy::from)?;
            Ok(())
        })
    }
}
