def bracket(points, target, max_gap=10):
    points = sorted(points, key=lambda x: x['quality'])
    for a, b in zip(points, points[1:]):
        if b['quality'] - a['quality'] > max_gap:
            continue
        if a['score'] <= target <= b['score'] and a['score'] < b['score']:
            return a, b
    return None

def interpolate(points, target, max_gap=10):
    pair = bracket(points, target, max_gap)
    if pair is None:
        return None
    a, b = pair
    return a['bytes'] + (b['bytes'] - a['bytes']) * (target - a['score']) / (b['score'] - a['score'])
