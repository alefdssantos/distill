def wrap_text(text, width):
    if not isinstance(text, str):
        raise ValueError('text must be a string')
    if isinstance(width, bool) or not isinstance(width, int) or width < 1:
        raise ValueError('width must be a positive integer')
    wrapped_lines = []
    for source_line in text.split('\n'):
        words = source_line.split()
        output = []
        current = ''
        for word in words:
            if len(word) > width:
                if current:
                    output.append(current)
                    current = ''
                while len(word) > width:
                    output.append(word[:width])
                    word = word[width:]
                current = word
            elif not current:
                current = word
            elif len(current) + 1 + len(word) <= width:
                current += ' ' + word
            else:
                output.append(current)
                current = word
        if current or not words:
            output.append(current)
        wrapped_lines.append('\n'.join(output))
    return '\n'.join(wrapped_lines)
