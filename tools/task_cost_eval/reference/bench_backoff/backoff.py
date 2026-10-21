import hashlib
import math


def retry_schedule(attempts, base, cap, seed):
    if isinstance(attempts, bool) or not isinstance(attempts, int) or attempts < 0:
        raise ValueError('attempts must be a non-negative int')
    for value in (base, cap):
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError('delays must be finite non-negative numbers')
    if base > cap or not isinstance(seed, str):
        raise ValueError('invalid cap or seed')
    result = []
    delay = float(base)
    for index in range(attempts):
        digest = hashlib.sha256((seed + ':' + str(index)).encode('utf-8')).digest()
        fraction = int.from_bytes(digest[:8], 'big') / 2**64
        result.append(delay * (0.5 + 0.5 * fraction))
        delay = min(cap, delay * 2)
    return result


def is_retryable(error):
    if not isinstance(error, BaseException):
        raise ValueError('error must be an exception instance')
    return isinstance(error, (TimeoutError, ConnectionError))
