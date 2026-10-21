class TokenBucket:
    def __init__(self, capacity, refill_rate):
        raise NotImplementedError('tokenbucket pending')

    def allow(self, key, now, cost=1):
        raise NotImplementedError('tokenbucket pending')
