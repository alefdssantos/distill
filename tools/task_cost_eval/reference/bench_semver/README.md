Implement strict SemVer 2.0 parsing and precedence in `semver.py` inside this slice, using only the Python standard library and Python 3.11+. Keep the existing function signatures.

`parse(text)` returns `(major, minor, patch, prerelease, build)`. The three core parts are non-negative integers and must not have leading zeros unless the part is exactly `0`. `prerelease` is a tuple of dot-separated identifiers, empty when absent; each identifier is a non-empty string of ASCII letters, digits or hyphens, numeric identifiers become `int` and must not have leading zeros, and alphanumeric identifiers stay `str`. `build` is a tuple of dot-separated identifiers, empty when absent; each is a non-empty string of ASCII letters, digits or hyphens and leading zeros are allowed. A missing build or prerelease part is an empty tuple. `parse('1.2.3')` is `(1, 2, 3, (), ())` and `parse('1.2.3-alpha.1+build.5')` is `(1, 2, 3, ('alpha', 1), ('build', '5'))`.

Validation is strict: `text` must be `str`; reject non-strings, empty strings, missing or extra core parts, signs, whitespace, non-ASCII characters, empty identifiers, leading or trailing dots, an empty prerelease or build after `-` or `+`, and numeric prerelease identifiers with leading zeros. Raise `ValueError` for every invalid input; never return `None`.

`compare(a, b)` accepts two version strings, parses them, and returns `-1`, `0` or `1`. Compare major, then minor, then patch numerically. A version without prerelease has higher precedence than one with prerelease. Compare prerelease identifiers left to right: numeric identifiers compare numerically, numeric identifiers have lower precedence than alphanumeric identifiers, alphanumeric identifiers compare by ordinary ASCII string order, and a shorter identifier list has lower precedence when all shared identifiers are equal. Build metadata is ignored. Raise `ValueError` when either argument is invalid.

Both functions are pure: do not mutate inputs and do not rely on dictionary iteration order. Case is significant; `Alpha` and `alpha` are different identifiers.

Run `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -v` from this slice.
