import math
from collections import OrderedDict


def _number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and (isinstance(value, int) or math.isfinite(value))


class LRUCache:
    def __init__(self, capacity, clock):
        if not isinstance(capacity, int) or isinstance(capacity, bool) or capacity <= 0 or not callable(clock):
            raise ValueError('invalid capacity or clock')
        self.capacity = capacity
        self.clock = clock
        self.entries = OrderedDict()

    def _validate_key(self, key):
        try:
            hash(key)
        except TypeError as error:
            raise ValueError('key must be hashable') from error

    def _now(self):
        now = self.clock()
        if not _number(now):
            raise ValueError('invalid clock value')
        return now

    def _prune(self, now):
        for key in list(self.entries):
            if self.entries[key][1] <= now:
                del self.entries[key]

    def get(self, key, default=None):
        self._validate_key(key)
        self._prune(self._now())
        if key not in self.entries:
            return default
        self.entries.move_to_end(key)
        return self.entries[key][0]

    def set(self, key, value, ttl):
        self._validate_key(key)
        if not _number(ttl) or ttl < 0:
            raise ValueError('invalid ttl')
        now = self._now()
        try:
            deadline = now + ttl
        except OverflowError as error:
            raise ValueError('invalid deadline') from error
        if not _number(deadline):
            raise ValueError('invalid deadline')
        self._prune(now)
        self.entries.pop(key, None)
        if ttl == 0:
            return None
        self.entries[key] = (value, deadline)
        if len(self.entries) > self.capacity:
            self.entries.popitem(last=False)
        return None

    def peek(self, key, default=None):
        self._validate_key(key)
        self._prune(self._now())
        return self.entries[key][0] if key in self.entries else default
