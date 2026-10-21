#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_toposort'),'toposort')
    except Exception as error:
        return finish(['import: '+repr(error)])
    graph = {'z': [], 'b': ['a','a'], 'a': [], 'é': ['z']}
    saved = copy.deepcopy(graph)
    attempt(failures,'dynamic lexical ties',lambda: subject.topological_sort(graph),['a','b','z','é'])
    attempt(failures,'empty graph',lambda: subject.topological_sort({}),[])
    attempt(failures,'chain',lambda: subject.topological_sort({'a': ['b'],'b': ['c'],'c': []}),['c','b','a'])
    attempt(failures,'unicode case order',lambda: subject.topological_sort({'é': [],'a': [],'A': [],'雪': []}),['A','a','é','雪'])
    attempt(failures,'empty node name',lambda: subject.topological_sort({'': [],'x': ['']}),['','x'])
    attempt(failures,'generator dependencies',lambda: subject.topological_sort({'b': iter(['a','a']),'a': iter([])}),['a','b'])
    attempt(failures,'insertion order irrelevant',lambda: subject.topological_sort(dict(reversed(list(graph.items())))),['a','b','z','é'])
    for index, invalid in enumerate(([], None, {1: []}, {'a': [1]}, {'a': ['missing']}, {'a': 'a'}, {'a': b'a'}, {'a': None})):
        attempt(failures,'invalid graph '+str(index),lambda: subject.topological_sort(invalid),raises=ValueError)
    def cycle_message(value, expected):
        try:
            subject.topological_sort(value)
        except ValueError as error:
            return 'cycle' in str(error) and expected in str(error)
        return False
    attempt(failures,'self cycle message',lambda: cycle_message({'x': ['x']},'x -> x'),True)
    attempt(failures,'closed cycle message',lambda: cycle_message({'z': [],'a': ['b'],'b': ['a']},'a -> b -> a'),True)
    attempt(failures,'sorted cycle selection',lambda: cycle_message({'a': ['c','b'],'b': ['a'],'c': ['a']},'a -> b -> a'),True)
    attempt(failures,'inputs unchanged',lambda: graph,saved)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
