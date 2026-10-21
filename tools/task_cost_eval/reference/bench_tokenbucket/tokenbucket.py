import math


def _number(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError('expected finite number')
    return value


class TokenBucket:
    def __init__(self, capacity, refill_rate):
        self.capacity = _number(capacity)
        self.refill_rate = _number(refill_rate)
        if capacity <= 0 or refill_rate < 0:
            raise ValueError('invalid bucket parameters')
        self._states = {}

    def allow(self, key, now, cost=1):
        if not isinstance(key, str):
            raise ValueError('key must be a string')
        now, cost = _number(now), _number(cost)
        if now < 0 or cost <= 0:
            raise ValueError('invalid time or cost')
        tokens, previous = self._states.get(key, (self.capacity, now))
        if now < previous:
            raise ValueError('time went backwards')
        tokens = min(self.capacity, tokens + (now - previous) * self.refill_rate)
        allowed = tokens >= cost
        if allowed:
            tokens -= cost
        self._states[key] = (tokens, now)
        return allowed
