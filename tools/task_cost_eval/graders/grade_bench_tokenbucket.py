#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_tokenbucket'),'tokenbucket')
    except Exception as error:
        return finish(['import: '+repr(error)])
    def sequence(capacity, rate, calls):
        bucket = subject.TokenBucket(capacity,rate)
        return [bucket.allow(*call) for call in calls]
    attempt(failures,'consume and refill',lambda: sequence(2,1,[('a',0,2),('a',0,1),('a',1,1)]),[True,False,True])
    attempt(failures,'independent empty and unicode keys',lambda: sequence(1,0,[('',5,1),('é',0,1),('',5,1),('É',0,1)]),[True,True,False,True])
    attempt(failures,'fractional boundary',lambda: sequence(1,0.5,[('a',0,1),('a',1,0.5),('a',1,0.01)]),[True,True,False])
    attempt(failures,'capacity clamp',lambda: sequence(2,1,[('a',0,2),('a',100,2),('a',100,1)]),[True,True,False])
    attempt(failures,'oversize denial preserves tokens',lambda: sequence(2,1,[('a',0,3),('a',0,2)]),[False,True])
    attempt(failures,'duplicate calls',lambda: sequence(2,0,[('a',0,1)]*3),[True,True,False])
    attempt(failures,'empty call list',lambda: sequence(1,0,[]),[])
    for index, pair in enumerate(((0,1),(-1,1),(1,-1),(True,1),(1,False),(float('nan'),1),(1,float('inf')),('1',1),(None,1))):
        attempt(failures,'invalid constructor '+str(index),lambda: subject.TokenBucket(*pair),raises=ValueError)
    for index, call in enumerate(((1,0,1),('a',-1,1),('a',True,1),('a',float('nan'),1),('a',float('inf'),1),('a',0,False),('a',0,0),('a',0,-1),('a',0,float('inf')),('a','0',1),('a',0,'1'))):
        attempt(failures,'invalid call '+str(index),lambda: subject.TokenBucket(1,1).allow(*call),raises=ValueError)
    def backwards():
        bucket = subject.TokenBucket(1,1)
        bucket.allow('a',5)
        bucket.allow('a',4)
    attempt(failures,'backward time',backwards,raises=ValueError)
    def denied_time():
        bucket = subject.TokenBucket(1,0)
        bucket.allow('a',5)
        bucket.allow('a',9)
        bucket.allow('a',8)
    attempt(failures,'denial records time',denied_time,raises=ValueError)
    def invalid_atomic():
        bucket = subject.TokenBucket(2,1)
        bucket.allow('a',2,1)
        for call in (('a',10,0),('a',1,1),('a',10,float('nan'))):
            try:
                bucket.allow(*call)
            except ValueError:
                pass
        return bucket.allow('a',2,1),bucket.allow('a',2,1)
    attempt(failures,'invalid calls preserve state',invalid_atomic,(True,False))
    def unchanged_calls():
        calls = [('a',0,1),('a',1,1)]
        saved = copy.deepcopy(calls)
        sequence(1,1,calls)
        return calls == saved
    attempt(failures,'caller input unchanged',unchanged_calls,True)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
