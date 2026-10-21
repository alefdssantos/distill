#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        template, = import_fresh(fixture_dir(argv,'bench_template'),'template')
    except Exception as error:
        return finish(['import: '+repr(error)])
    attempt(failures,'escaped value',lambda: template.render('{{x}}',{'x':'&<>"\''}),'&amp;&lt;&gt;&quot;&#x27;')
    attempt(failures,'raw value',lambda: template.render('{{{x}}}',{'x':'<b>&</b>'}),'<b>&</b>')
    attempt(failures,'dotted lookup',lambda: template.render('{{person.name}}',{'person':{'name':'Ada'}}),'Ada')
    attempt(failures,'empty template',lambda: template.render('',{}),'')
    attempt(failures,'literal unicode',lambda: template.render('雪 and café',{}),'雪 and café')
    attempt(failures,'missing policy',lambda: template.render('a{{missing}}b{{{other}}}c',{}),'abc')
    attempt(failures,'nonmapping intermediate',lambda: template.render('{{x.real}}/{{xs.thing}}',{'x':1,'xs':[]}),'/')
    attempt(failures,'None policy',lambda: template.render('{{x}}:{{{x}}}',{'x':None}),':')
    attempt(failures,'unicode and case',lambda: template.render('{{名}}/{{Name}}/{{name}}',{'名':'雪','Name':'A','name':'b'}),'雪/A/b')
    attempt(failures,'outer unicode whitespace',lambda: template.render('{{\u2003x\t}}',{'x':2}),'2')
    attempt(failures,'duplicates and order',lambda: template.render('{{b}}{{a}}{{b}}',{'a':1,'b':2}),'212')
    attempt(failures,'no recursive rendering',lambda: template.render('{{x}}',{'x':'{{y}}','y':'wrong'}),'{{y}}')
    attempt(failures,'adjacent mixed tokens',lambda: template.render('{{x}}{{{x}}}',{'x':'<'}),'&lt;<')
    for invalid in ['{','}','{{x}','{{x}}}','{{{x}}','{{{{x}}}}','{{}}','{{{}}}','{{x..y}}','{{x y}}','{{x.0}}','{{!x}}','{{#x}}','a{b}']:
        attempt(failures,'invalid syntax '+repr(invalid),lambda: template.render(invalid,{}),raises=ValueError)
    for invalid in [None,True,12,[],b'{{x}}']:
        attempt(failures,'invalid template '+repr(invalid),lambda: template.render(invalid,{}),raises=ValueError)
    for invalid in [None,[],1,'context']:
        attempt(failures,'invalid context '+repr(invalid),lambda: template.render('',invalid),raises=ValueError)
    from collections import UserDict
    attempt(failures,'mapping protocol',lambda: template.render('{{x.y}}',UserDict(x=UserDict(y='yes'))),'yes')
    context = {'person':{'name':'<Ada>'},'unused':[1,2]}
    saved = copy.deepcopy(context)
    attempt(failures,'render nested input',lambda: template.render('{{person.name}}',context),'&lt;Ada&gt;')
    attempt(failures,'input unchanged',lambda: context,saved)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
