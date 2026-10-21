#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_configmerge'),'config_merge')
    except Exception as error:
        return finish(['import: '+repr(error)])
    base = {'a': {'x': [1], 'keep': 2}, 'n': 3}
    override = {'a': {'x': [2], 'new': 4}, 'n': None}
    saved = copy.deepcopy((base, override))
    attempt(failures,'nested merge',lambda: subject.deep_merge(base,override),{'a': {'x': [2], 'keep': 2, 'new': 4}, 'n': None})
    attempt(failures,'empty merge',lambda: subject.deep_merge({},{}),{})
    attempt(failures,'lists replace',lambda: subject.deep_merge({'x': [1,1]},{'x': [2,2]}),{'x': [2,2]})
    attempt(failures,'dict replaces scalar',lambda: subject.deep_merge({'x': 1},{'x': {'y': 2}}),{'x': {'y': 2}})
    attempt(failures,'scalar replaces dict',lambda: subject.deep_merge({'x': {'y': 2}},{'x': 1}),{'x': 1})
    attempt(failures,'None replaces dict',lambda: subject.deep_merge({'x': {}},{'x': None}),{'x': None})
    attempt(failures,'unicode exact keys',lambda: subject.deep_merge({'A': 1,'é': 2},{'a': 3}),{'A': 1,'é': 2,'a': 3})
    attempt(failures,'override duplicate key',lambda: subject.deep_merge({'x': 1},{'x': 3}),{'x': 3})
    for index, invalid in enumerate((None, [], 'x', True)):
        attempt(failures,'invalid base '+str(index),lambda: subject.deep_merge(invalid,{}),raises=ValueError)
        attempt(failures,'invalid override '+str(index),lambda: subject.deep_merge({},invalid),raises=ValueError)
    def detached():
        result = subject.deep_merge(base,override)
        result['a']['x'].append(9)
        result['a']['keep'] = 0
        return base,override
    attempt(failures,'detached merge',detached,saved)
    attempt(failures,'merge inputs unchanged',lambda: (base,override),saved)
    config = {'a': {'b': None, 'é': [1]}, 'A': 7, 'list': [2]}
    saved_config = copy.deepcopy(config)
    attempt(failures,'get None',lambda: subject.get_path(config,'a.b'),None)
    attempt(failures,'get unicode',lambda: subject.get_path(config,'a.é'),[1])
    attempt(failures,'get exact case',lambda: subject.get_path(config,'A'),7)
    attempt(failures,'get stored identity',lambda: subject.get_path(config,'a') is config['a'],True)
    for index, path in enumerate(('',None,1,True,'.a','a.','a..b')):
        attempt(failures,'invalid path '+str(index),lambda: subject.get_path(config,path),raises=ValueError)
    for path in ('missing','a.B','a.b.x','list.0'):
        attempt(failures,'missing '+path,lambda: subject.get_path(config,path),raises=KeyError)
    attempt(failures,'empty config',lambda: subject.get_path({},'x'),raises=KeyError)
    attempt(failures,'invalid config',lambda: subject.get_path([], 'x'),raises=ValueError)
    attempt(failures,'getter input unchanged',lambda: config,saved_config)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
