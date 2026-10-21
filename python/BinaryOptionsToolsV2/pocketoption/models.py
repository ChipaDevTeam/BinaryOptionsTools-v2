"""Typed data models returned by the PocketOption client API."""

from typing import Any, Dict, Optional, Tuple


class Trade(dict):
    """A trade order or result.

    Behaves exactly like the raw ``dict`` returned by the platform, so existing
    mapping access (``trade["profit"]``) keeps working, while also exposing each
    field as an attribute (``trade.profit``) and the outcome as readable flags
    (``trade.is_win``).

    A freshly placed order carries ``asset`` and ``direction``; once settled it
    also carries ``profit`` and ``result`` (``"win"``, ``"loss"`` or ``"draw"``).
    """

    def __getattr__(self, name: str) -> Any:
        try:
            return self[name]
        except KeyError as exc:
            raise AttributeError(name) from exc

    @property
    def result(self) -> Optional[str]:
        """Settled outcome: ``"win"``, ``"loss"``, ``"draw"`` or ``None``."""
        return self.get("result")

    @property
    def profit(self) -> Optional[float]:
        """Profit/loss amount once settled, otherwise ``None``."""
        value = self.get("profit")
        return None if value is None else float(value)

    @property
    def is_win(self) -> bool:
        """``True`` when the settled outcome is a win."""
        return self.get("result") == "win"

    @property
    def is_loss(self) -> bool:
        """``True`` when the settled outcome is a loss."""
        return self.get("result") == "loss"

    @property
    def is_draw(self) -> bool:
        """``True`` when the settled outcome is a draw (stake returned)."""
        return self.get("result") == "draw"

    @property
    def is_settled(self) -> bool:
        """``True`` once the trade has an outcome."""
        return "result" in self

    def to_dict(self) -> Dict[str, Any]:
        """Return a plain ``dict`` copy, detached from this model."""
        return dict(self)


TradeResult = Tuple[str, Trade]
"""Return type of :meth:`buy`/:meth:`sell`: ``(trade_id, trade)``."""
