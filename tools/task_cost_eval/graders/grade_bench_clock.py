#!/usr/bin/env python3
import copy
import sys
from fractions import Fraction
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        clock, = import_fresh(fixture_dir(argv,'bench_clock'),'clock')
    except Exception as error:
        return finish(['import: '+repr(error)])
    for text, expected in [('9:05 am',545),('23:59',1439),('12:00 AM',0),('12:00 pM',720),('1:02 PM',782),('0:00',0),('00:00',0),('09:05',545)]:
        attempt(failures,'parse '+text,lambda: clock.parse_time(text),expected)
    for invalid in ['',None,True,123,'24:00','00:01 AM','13:00 pm','1:60','1:2',' 9:05','9:05 ','９:05','9:05am','9:05 AM\n','1:00 XM']:
        attempt(failures,'invalid time '+repr(invalid),lambda: clock.parse_time(invalid),raises=ValueError)
    attempt(failures,'format midnight',lambda: clock.format_time(0),'00:00')
    attempt(failures,'format last minute',lambda: clock.format_time(1439),'23:59')
    for invalid in [-1,1440,True,1.0,'1',None]:
        attempt(failures,'invalid minutes '+repr(invalid),lambda: clock.format_time(invalid),raises=ValueError)
    attempt(failures,'forward wrap',lambda: clock.add_minutes('23:59',2),'00:01')
    attempt(failures,'backward wrap',lambda: clock.add_minutes('12:00 AM',-1),'23:59')
    attempt(failures,'large delta',lambda: clock.add_minutes('1:01',1440*10**30),'01:01')
    for invalid in [True,1.5,'2',None]:
        attempt(failures,'invalid delta '+repr(invalid),lambda: clock.add_minutes('0:00',invalid),raises=ValueError)
    text = '9:05 am'
    attempt(failures,'duplicate pure calls',lambda: (clock.add_minutes(text,0),clock.add_minutes(text,0),text),('09:05','09:05','9:05 am'))
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
