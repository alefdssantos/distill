#!/usr/bin/env python3
import copy
import sys
import ast
import hashlib
from pathlib import Path
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_backoff'),'backoff')
    except Exception as error:
        return finish(['import: '+repr(error)])
    def expected(count, base, cap, seed):
        result, delay = [], float(base)
        for index in range(count):
            digest = hashlib.sha256((seed+':'+str(index)).encode('utf-8')).digest()
            result.append(delay * (0.5 + 0.5 * (int.from_bytes(digest[:8],'big') / 2**64)))
            delay = min(cap,delay*2)
        return result
    attempt(failures,'exact seeded schedule',lambda: subject.retry_schedule(6,1,3,'seed'),expected(6,1,3,'seed'))
    attempt(failures,'empty schedule',lambda: subject.retry_schedule(0,1,3,''),[])
    attempt(failures,'zero duplicate delays',lambda: subject.retry_schedule(4,0,0,''),[0.0]*4)
    attempt(failures,'cap boundary',lambda: subject.retry_schedule(4,2,2,'A'),expected(4,2,2,'A'))
    attempt(failures,'fractional base',lambda: subject.retry_schedule(4,0.25,1,'é'),expected(4,0.25,1,'é'))
    attempt(failures,'unicode case sensitive seed',lambda: subject.retry_schedule(3,1,8,'É'),expected(3,1,8,'É'))
    attempt(failures,'long capped schedule',lambda: subject.retry_schedule(1100,1,2,'x'),expected(1100,1,2,'x'))
    attempt(failures,'repeat call deterministic',lambda: subject.retry_schedule(6,1,3,'seed'),expected(6,1,3,'seed'))
    for index, args in enumerate(((-1,1,2,'x'),(True,1,2,'x'),(1.0,1,2,'x'),(1,-1,2,'x'),(1,1,-2,'x'),(1,3,2,'x'),(1,True,2,'x'),(1,1,False,'x'),(1,float('nan'),2,'x'),(1,1,float('inf'),'x'),(1,'1',2,'x'),(0,1,2,None))):
        attempt(failures,'invalid schedule '+str(index),lambda: subject.retry_schedule(*args),raises=ValueError)
    for index, error in enumerate((TimeoutError(),ConnectionError(),ConnectionResetError(),BrokenPipeError())):
        attempt(failures,'retryable '+str(index),lambda: subject.is_retryable(error),True)
    for index, error in enumerate((ValueError(),OSError(),KeyboardInterrupt(),Exception('TimeoutError'))):
        attempt(failures,'not retryable '+str(index),lambda: subject.is_retryable(error),False)
    class Child(TimeoutError):
        pass
    attempt(failures,'subclass retryable',lambda: subject.is_retryable(Child()),True)
    for index, error in enumerate((None,'TimeoutError',TimeoutError,1)):
        attempt(failures,'invalid error '+str(index),lambda: subject.is_retryable(error),raises=ValueError)
    error = TimeoutError('keep')
    error.extra = [1]
    def unchanged():
        subject.is_retryable(error)
        return error.args,error.__dict__
    attempt(failures,'exception unchanged',unchanged,(('keep',),{'extra': [1]}))
    attempt(failures,'no random module',lambda: any(isinstance(node,(ast.Import,ast.ImportFrom)) and ((isinstance(node,ast.ImportFrom) and (node.module or '').split('.')[0] == 'random') or (isinstance(node,ast.Import) and any(alias.name.split('.')[0] == 'random' for alias in node.names))) for node in ast.walk(ast.parse(Path(subject.__file__).read_text()))),False)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
