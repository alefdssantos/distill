from collections.abc import Mapping
from html import escape


def render(template, context):
    if not isinstance(template, str) or not isinstance(context, Mapping):
        raise ValueError('invalid template or context')
    parts = []
    index = 0
    while index < len(template):
        char = template[index]
        if char == '}':
            raise ValueError('unbalanced braces')
        if char != '{':
            parts.append(char)
            index += 1
            continue
        raw = template.startswith('{{{', index)
        opening = '{{{' if raw else '{{'
        closing = '}}}' if raw else '}}'
        if not template.startswith(opening, index):
            raise ValueError('unbalanced braces')
        end = template.find(closing, index + len(opening))
        if end < 0:
            raise ValueError('unbalanced braces')
        name = template[index + len(opening):end].strip()
        segments = name.split('.')
        if not all(segment.isidentifier() for segment in segments):
            raise ValueError('invalid variable name')
        value = context
        for segment in segments:
            if not isinstance(value, Mapping) or segment not in value:
                value = None
                break
            value = value[segment]
        text = '' if value is None else str(value)
        parts.append(text if raw else escape(text, quote=True))
        index = end + len(closing)
    return ''.join(parts)
