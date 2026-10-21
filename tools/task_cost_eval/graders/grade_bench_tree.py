#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_tree'),'tree')
    except Exception as error:
        return finish(['import: '+repr(error)])
    paths = ['z','a/c','a/b','a','é/雪','A']
    saved = copy.deepcopy(paths)
    expected = {'z': {},'a': {'c': {},'b': {}},'é': {'雪': {}},'A': {}}
    attempt(failures,'nested build',lambda: subject.build_tree(paths),expected)
    attempt(failures,'empty build',lambda: subject.build_tree([]),{})
    attempt(failures,'generator build',lambda: subject.build_tree(iter(paths)),expected)
    attempt(failures,'input order irrelevant',lambda: subject.build_tree(list(reversed(paths))),expected)
    attempt(failures,'parent first',lambda: subject.build_tree(['a','a/b']),{'a': {'b': {}}})
    attempt(failures,'literal spaces and backslash',lambda: subject.build_tree([' a /b\\c']),{' a ': {'b\\c': {}}})
    for index, invalid in enumerate((None,'a',b'a',[1],[''],['/a'],['a/'],['a//b'],['.'],['a/..'],['a\n'],['a\r'],['a','a'],['a/b','a','a'])):
        attempt(failures,'invalid paths '+str(index),lambda: subject.build_tree(invalid),raises=ValueError)
    tree = copy.deepcopy(expected)
    saved_tree = copy.deepcopy(tree)
    attempt(failures,'sorted rendering',lambda: subject.render_tree(tree),'A\na\n  b\n  c\nz\né\n  雪')
    attempt(failures,'prefix every line',lambda: subject.render_tree({'a': {'b': {}},'z': {}},'> '),'> a\n>   b\n> z')
    attempt(failures,'empty rendering',lambda: subject.render_tree({},'prefix'),'')
    attempt(failures,'prefix does not filter',lambda: subject.render_tree({'a': {}},'z'),'za')
    attempt(failures,'reverse insertion rendering',lambda: subject.render_tree({'z': {},'A': {}}),'A\nz')
    for index, invalid in enumerate(([], {'a': []},{1: {}},{'': {}},{'a/b': {}},{'.': {}},{'..': {}},{'a\n': {}},{'a\r': {}})):
        attempt(failures,'invalid tree '+str(index),lambda: subject.render_tree(invalid),raises=ValueError)
    for index, prefix in enumerate((None,1,'a\n','a\r')):
        attempt(failures,'invalid prefix '+str(index),lambda: subject.render_tree({},prefix),raises=ValueError)
    cyclic = {}
    cyclic['a'] = cyclic
    attempt(failures,'cycle rejected',lambda: subject.render_tree(cyclic),raises=ValueError)
    shared = {'x': {}}
    attempt(failures,'shared acyclic subtrees',lambda: subject.render_tree({'a': shared,'b': shared}),'a\n  x\nb\n  x')
    attempt(failures,'path input unchanged',lambda: paths,saved)
    attempt(failures,'tree input unchanged',lambda: tree,saved_tree)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
