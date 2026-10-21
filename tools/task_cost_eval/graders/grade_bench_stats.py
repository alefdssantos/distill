#!/usr/bin/env python3
import copy
import sys
from fractions import Fraction
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        stats, = import_fresh(fixture_dir(argv,'bench_stats'),'stats')
    except Exception as error:
        return finish(['import: '+repr(error)])
    expected = dict(count=3,mean=Fraction(2),min=1,max=3,variance=Fraction(2,3))
    data = [3,1,2]
    attempt(failures,'summary and unordered extrema',lambda: stats.summarize(data),expected)
    attempt(failures,'empty',lambda: stats.summarize([]),dict(count=0,mean=None,min=None,max=None,variance=None))
    attempt(failures,'singleton',lambda: stats.summarize([-7]),dict(count=1,mean=Fraction(-7),min=-7,max=-7,variance=Fraction(0)))
    attempt(failures,'duplicates',lambda: stats.summarize([4,4,4]),dict(count=3,mean=Fraction(4),min=4,max=4,variance=Fraction(0)))
    attempt(failures,'rational mean',lambda: stats.summarize([0,1]),dict(count=2,mean=Fraction(1,2),min=0,max=1,variance=Fraction(1,4)))
    attempt(failures,'negative samples',lambda: stats.summarize([-3,-1])['variance'],Fraction(1))
    attempt(failures,'large exact integers',lambda: stats.summarize([10**100,10**100+1])['variance'],Fraction(1,4))
    attempt(failures,'fraction types',lambda: tuple(type(stats.summarize([2])[key]) is Fraction for key in ('mean','variance')),(True,True))
    class Once:
        def __init__(self):
            self.used = False
        def __iter__(self):
            if self.used:
                raise AssertionError('revisited iterable')
            self.used = True
            yield from [1,2,3]
    attempt(failures,'one pass',lambda: stats.summarize(Once()),expected)
    attempt(failures,'generator',lambda: stats.summarize(x for x in [1,2,3]),expected)
    for invalid in [[True],[1.0],['雪'],[None],[Fraction(1)],None,1]:
        attempt(failures,'invalid samples '+repr(invalid),lambda: stats.summarize(invalid),raises=ValueError)
    def detached():
        result = stats.summarize(data)
        result['count'] = 99
        return stats.summarize(data)
    attempt(failures,'fresh result',detached,expected)
    attempt(failures,'input unchanged',lambda: data,[3,1,2])
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
