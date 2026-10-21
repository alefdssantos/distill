#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        subject, = import_fresh(fixture_dir(argv,'bench_bytes'),'sizes')
    except Exception as error:
        return finish(['import: '+repr(error)])
    for text, value in (('0 B',0),('10 B',10),('2 MB',2000000),('1.5 KiB',1536),('0.5 KiB',512),('1.1 KB',1100),('  0001.000 B\t',1),('1 TB',10**12),('1 TiB',1024**4),('1 GiB',1024**3),('1 MiB',1024**2),('1 GB',10**9),('0.001 KB',1),('9007199254740993 B',9007199254740993)):
        attempt(failures,'parse '+repr(text),lambda: subject.parse_size(text),value)
    for index, text in enumerate((None,True,1,b'1 B','','1','-1 B','+1 B','1e3 B','1_000 B','1,000 B','1\tB','1  B','1 kb','1 K','1 XB','١ B','１ B','.5 KB','1. B','0.5 B','1.1 KiB','1 B extra','1 B\n2 B')):
        attempt(failures,'invalid parse '+str(index),lambda: subject.parse_size(text),raises=ValueError)
    for value, text in ((0,'0 B'),(10,'10 B'),(999,'999 B'),(1000,'1 KB'),(1005,'1.01 KB'),(1536,'1.54 KB'),(2000000,'2 MB'),(999994,'999.99 KB'),(999995,'1 MB'),(999999,'1 MB'),(10**9,'1 GB'),(10**12,'1 TB'),(10**15,'1000 TB'),(1234500,'1.23 MB')):
        attempt(failures,'format '+str(value),lambda: subject.format_size(value),text)
    for index, value in enumerate((True,False,-1,1.0,'1',None,float('inf'),float('nan'))):
        attempt(failures,'invalid format '+str(index),lambda: subject.format_size(value),raises=ValueError)
    attempt(failures,'duplicate parse calls',lambda: [subject.parse_size('1 KB') for _ in range(2)],[1000,1000])
    inputs = ['1 B','2 KB','1 MiB']
    saved = copy.deepcopy(inputs)
    def unchanged():
        for text in inputs:
            subject.parse_size(text)
        return inputs
    attempt(failures,'inputs unchanged',unchanged,saved)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
