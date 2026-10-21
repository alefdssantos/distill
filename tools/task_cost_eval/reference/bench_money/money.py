def allocate(cents, weights):
    if isinstance(cents, bool) or not isinstance(cents, int) or cents < 0:
        raise ValueError('cents must be a non-negative int')
    try:
        values = list(weights)
    except TypeError:
        raise ValueError('weights must be iterable') from None
    if not values:
        raise ValueError('weights must not be empty')
    for weight in values:
        if isinstance(weight, bool) or not isinstance(weight, int) or weight < 0:
            raise ValueError('weights must be non-negative ints')
    total = sum(values)
    if total == 0:
        raise ValueError('weights must have a positive total')
    shares = []
    remainders = []
    for index, weight in enumerate(values):
        base, remainder = divmod(cents * weight, total)
        shares.append(base)
        remainders.append((remainder, index))
    leftover = cents - sum(shares)
    for _, index in sorted(remainders, key=lambda item: (-item[0], item[1]))[:leftover]:
        shares[index] += 1
    return shares
