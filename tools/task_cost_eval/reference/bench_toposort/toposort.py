import heapq


def topological_sort(dependencies):
    if not isinstance(dependencies, dict) or any(not isinstance(node, str) for node in dependencies):
        raise ValueError('dependencies must be a dict of string nodes')
    graph = {}
    for node, values in dependencies.items():
        if isinstance(values, (str, bytes)):
            raise ValueError('dependencies must be iterables of nodes')
        try:
            values = list(values)
        except TypeError:
            raise ValueError('dependencies must be iterable') from None
        if any(not isinstance(value, str) or value not in dependencies for value in values):
            raise ValueError('unknown or non-string dependency')
        graph[node] = set(values)
    pending = {node: len(values) for node, values in graph.items()}
    children = {node: set() for node in graph}
    for node, values in graph.items():
        for value in values:
            children[value].add(node)
    ready = [node for node in graph if not pending[node]]
    heapq.heapify(ready)
    result = []
    while ready:
        node = heapq.heappop(ready)
        result.append(node)
        for child in sorted(children[node]):
            pending[child] -= 1
            if not pending[child]:
                heapq.heappush(ready, child)
    if len(result) != len(graph):
        active, done = [], set()
        def visit(node):
            if node in active:
                cycle = active[active.index(node):] + [node]
                raise ValueError('cycle: ' + ' -> '.join(cycle))
            if node in done:
                return
            active.append(node)
            for value in sorted(graph[node]):
                visit(value)
            active.pop()
            done.add(node)
        for node in sorted(graph):
            visit(node)
    return result
