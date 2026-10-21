from fractions import Fraction


def summarize(samples):
    count = total = squares = 0
    low = high = None
    try:
        iterator = iter(samples)
    except TypeError as error:
        raise ValueError('samples must be iterable') from error
    for sample in iterator:
        if not isinstance(sample, int) or isinstance(sample, bool):
            raise ValueError('samples must be integers')
        count += 1
        total += sample
        squares += sample * sample
        low = sample if low is None else min(low, sample)
        high = sample if high is None else max(high, sample)
    mean = Fraction(total, count) if count else None
    variance = Fraction(squares, count) - mean * mean if count else None
    return dict(count=count, mean=mean, min=low, max=high, variance=variance)
