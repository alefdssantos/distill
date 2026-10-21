from copy import deepcopy


def deep_merge(base, override):
    if not isinstance(base, dict) or not isinstance(override, dict):
        raise ValueError('merge inputs must be dicts')
    result = deepcopy(base)
    for key, value in override.items():
        if key in base and isinstance(base[key], dict) and isinstance(value, dict):
            result[key] = deep_merge(base[key], value)
        else:
            result[key] = deepcopy(value)
    return result


def get_path(config, path):
    if not isinstance(config, dict):
        raise ValueError('config must be a dict')
    if not isinstance(path, str) or not path or any(not part for part in path.split('.')):
        raise ValueError('path must have nonempty dotted components')
    value = config
    for part in path.split('.'):
        if not isinstance(value, dict) or part not in value:
            raise KeyError(path)
        value = value[part]
    return value
