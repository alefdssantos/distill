#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        matrix, = import_fresh(fixture_dir(argv,'bench_matrix'),'matrix')
    except Exception as error:
        return finish(['import: '+repr(error)])
    data = [[2, 0, 4], [0, 1, 3], [2, 0, -1], [0, 1, -3]]
    saved = copy.deepcopy(data)
    attempt(failures,'duplicate cancellation and sorting',lambda: matrix.build(data),[(2,0,3)])
    attempt(failures,'empty build',lambda: matrix.build([]),[])
    attempt(failures,'build coordinate order',lambda: matrix.build([(2,1,5),(0,3,2),(0,1,4)]),[(0,1,4),(0,3,2),(2,1,5)])
    attempt(failures,'product coordinate order',lambda: matrix.multiply([(2,0,3),(0,0,2)],[(0,4,5),(0,1,7)]),[(0,1,14),(0,4,10),(2,1,21),(2,4,15)])
    attempt(failures,'generator',lambda: matrix.build(iter([(3,2,0),(0,0,-5)])),[(0,0,-5)])
    attempt(failures,'transpose order',lambda: matrix.transpose([(0,2,4),(1,0,5)]),[(0,1,5),(2,0,4)])
    attempt(failures,'empty transpose',lambda: matrix.transpose([]),[])
    attempt(failures,'product accumulation',lambda: matrix.multiply([(0,0,2),(0,1,3)],[(0,2,4),(1,2,5)]),[(0,2,23)])
    attempt(failures,'unmatched product',lambda: matrix.multiply([(0,2,1)],[(1,0,7)]),[])
    attempt(failures,'empty product',lambda: matrix.multiply([],[]),[])
    attempt(failures,'product cancellation',lambda: matrix.multiply([(0,0,1),(0,1,1)],[(0,0,2),(1,0,-2)]),[])
    attempt(failures,'large integer',lambda: matrix.build([(0,0,10**80)]),[(0,0,10**80)])
    for label, invalid in [('negative row',[(-1,0,1)]),('negative col',[(0,-1,1)]),('bool index',[(True,0,1)]),('bool value',[(0,0,True)]),('float value',[(0,0,1.0)]),('unicode value',[(0,0,'雪')]),('malformed',[(0,1)]),('noniterable',None)]:
        attempt(failures,label,lambda: matrix.build(invalid),raises=ValueError)
    attempt(failures,'transpose validation',lambda: matrix.transpose([(0,-1,2)]),raises=ValueError)
    attempt(failures,'both operands validated',lambda: matrix.multiply([],[(0,0,False)]),raises=ValueError)
    def detached():
        result = matrix.build(data)
        result.append((9,9,9))
        return data
    attempt(failures,'detached result',detached,saved)
    attempt(failures,'inputs unchanged',lambda: data,saved)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
