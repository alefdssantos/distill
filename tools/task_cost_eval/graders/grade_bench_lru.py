#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        lru, = import_fresh(fixture_dir(argv,'bench_lru'),'lru')
    except Exception as error:
        return finish(['import: '+repr(error)])
    def scenario(actions, capacity=2):
        now = [0]
        cache = lru.LRUCache(capacity, lambda: now[0])
        result = []
        for action in actions:
            if action[0] == 'time':
                now[0] = action[1]
            else:
                result.append(getattr(cache, action[0])(*action[1:]))
        return result
    attempt(failures,'empty misses',lambda: scenario([('get','x'),('peek','x','missing')]),[None,'missing'])
    attempt(failures,'set and hit',lambda: scenario([('set','a',1,10),('get','a')]),[None,1])
    attempt(failures,'get promotes at tied times',lambda: scenario([('set','z',1,10),('set','a',2,10),('get','z'),('set','b',3,10),('peek','a'),('peek','z')]),[None,None,1,None,None,1])
    attempt(failures,'peek does not promote',lambda: scenario([('set','z',1,10),('set','a',2,10),('peek','z'),('set','b',3,10),('get','z')]),[None,None,1,None,None])
    attempt(failures,'exact expiry',lambda: scenario([('set','a',1,2),('time',1.5),('get','a'),('time',2),('peek','a')]),[None,1,None])
    attempt(failures,'prune all before eviction',lambda: scenario([('set','a',1,100),('set','b',2,1),('time',2),('set','c',3,100),('get','a')]),[None,None,None,1])
    attempt(failures,'duplicate refresh',lambda: scenario([('set','a',1,1),('time',0.5),('set','a',2,10),('time',2),('get','a')]),[None,None,2])
    attempt(failures,'duplicate promotes',lambda: scenario([('set','a',1,10),('set','b',2,10),('set','a',3,10),('set','c',4,10),('peek','b'),('peek','a')]),[None,None,None,None,None,3])
    attempt(failures,'zero ttl removes',lambda: scenario([('set','a',1,10),('set','a',2,0),('get','a')]),[None,None,None])
    attempt(failures,'zero ttl never evicts',lambda: scenario([('set','a',1,10),('set','b',2,0),('get','a')],1),[None,None,1])
    attempt(failures,'None value',lambda: scenario([('set','a',None,10),('get','a','missing')]),[None,None])
    attempt(failures,'unicode case exact',lambda: scenario([('set','é',1,10),('set','É',2,10),('get','é'),('get','É')]),[None,None,1,2])
    for capacity in [0,-1,True,1.0,'2']:
        attempt(failures,'invalid capacity '+repr(capacity),lambda: lru.LRUCache(capacity,lambda: 0),raises=ValueError)
    attempt(failures,'invalid clock',lambda: lru.LRUCache(1,None),raises=ValueError)
    for ttl in [-1,True,'1',None,float('nan'),float('inf')]:
        attempt(failures,'invalid ttl '+repr(ttl),lambda: lru.LRUCache(1,lambda: 0).set('a',1,ttl),raises=ValueError)
    for method in ['get','peek','set']:
        attempt(failures,'unhashable '+method,lambda: getattr(lru.LRUCache(1,lambda: 0),method)([],*([1,2] if method=='set' else [])),raises=ValueError)
    for now in [True,None,'0',float('nan'),float('inf')]:
        attempt(failures,'invalid clock reading '+repr(now),lambda: lru.LRUCache(1,lambda: now).get('x'),raises=ValueError)
    attempt(failures,'deadline overflow',lambda: lru.LRUCache(1,lambda: 1e308).set('a',1,1e308),raises=ValueError)
    def observed():
        calls = []
        def clock():
            calls.append(0)
            return 0
        cache = lru.LRUCache(1,clock)
        before = len(calls)
        cache.set('a',1,2)
        cache.get('a')
        cache.peek('a')
        try:
            cache.set('b',2,-1)
        except ValueError:
            pass
        return before,len(calls),cache.get('a')
    attempt(failures,'clock calls and rejected set atomicity',observed,(0,3,1))
    def immutable():
        class Key:
            def __init__(self):
                self.parts = ['original']
        key = Key()
        value = ['value']
        cache = lru.LRUCache(1,lambda: 0)
        cache.set(key,value,10)
        same = cache.get(key) is value
        cache.set('other',0,10)
        return key.parts,value,same
    attempt(failures,'caller key and value unchanged',immutable,(['original'],['value'],True))
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
