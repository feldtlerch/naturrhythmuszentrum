# SPDX-License-Identifier: AGPL-3.0-only
"""Integer-only reproduction of clamp golden values; no CAS or checker imports."""
from fractions import Fraction as Q
import json
import sys

SIGN = 1 << 63
MAX = 0x7fefffffffffffff


def power(e):
    return Q(1 << e) if e >= 0 else Q(1, 1 << -e)


def value(bits):
    exponent = (bits >> 52) & 2047
    mantissa = bits & ((1 << 52) - 1)
    assert exponent != 2047, "finite bits"
    if exponent:
        mantissa += 1 << 52
    return (-1 if bits & SIGN else 1) * mantissa * power(exponent - 1075 if exponent else -1074)


def nearest(a):
    sign = SIGN if a < 0 else 0
    a = abs(a)
    if not a:
        return 0
    exponent = a.numerator.bit_length() - a.denominator.bit_length()
    if a < power(exponent):
        exponent -= 1
    scale = max(exponent - 52, -1074)
    scaled = a / power(scale)
    quotient, remainder = divmod(scaled.numerator, scaled.denominator)
    if 2 * remainder > scaled.denominator or (2 * remainder == scaled.denominator and quotient % 2):
        quotient += 1
    if quotient == 1 << 53:
        quotient >>= 1
        scale += 1
    if scale > 971:
        return None
    bits = quotient if quotient < 1 << 52 else ((scale + 1075) << 52) + quotient - (1 << 52)
    return bits | (sign if bits else 0)


def wall(a):
    c = nearest(a)
    if c is None:
        return None, None
    distance = abs(value(c) - a)
    t = nearest(distance)
    if value(t) < distance:
        t += 1
    return c, t


def neighbor(bits, up):
    if bits & ~SIGN == 0:
        answer = 1 if up else SIGN | 1
    elif bool(bits & SIGN) == up:
        answer = bits - 1
    else:
        answer = bits + 1
    return answer if answer & ~SIGN <= MAX else None


def outward(bits, up):
    if neighbor(bits, not up) is None:
        return None
    return neighbor(bits, up)


def derive(row):
    x, r = (value(int(row['input'][k], 16)) for k in ('x', 'r'))
    a = Q(row['wall']['a'])
    c_bits, t_bits = wall(a)
    supplied_c, supplied_t = (int(row['wall'][k], 16) for k in ('c', 't'))
    if c_bits != supplied_c:
        return 'ClampWallCenterMismatch', None
    c, t = value(c_bits), value(supplied_t)
    if t < abs(c - a):
        return 'ClampWallNotEnclosed', None
    if t_bits != supplied_t:
        return 'ClampWallTotalMismatch', None
    lo = nearest(x-r)
    hi = nearest(x+r)
    lo = 0 if x == r else (None if lo is None else outward(lo, False))
    if not x or not r:
        hi = nearest(x+r)
    elif hi is not None:
        hi = outward(hi, True)
    if lo is None or hi is None:
        return 'UnboundedImage', None
    return 'Enclosure', {
        'image': [str(max(value(lo), c-t)), str(max(value(hi), c+t))],
        'nominal': [str(max(x, c-t)), str(max(x, c+t))],
        'center': str(max(x, c)), 'error': str(t),
    }


def check(path):
    assert nearest(value(MAX)) == MAX, 'nearest largest finite value'
    assert nearest(-value(MAX)) == SIGN | MAX, 'nearest negative largest value'
    assert outward(MAX, False) is None, 'opposite upper neighbour'
    assert outward(SIGN | MAX, True) is None, 'opposite lower neighbour'
    assert nearest(Q(2) - power(-53)) == 0x4000000000000000, 'nearest carry'
    assert nearest(power(1024)) is None, 'nearest finite range'
    assert nearest(power(1024) - power(970)) is None, 'nearest overflow tie'
    payload = json.load(open(path, encoding='utf-8'))
    for row in payload['entries']:
        expected, golden = derive(row)
        assert expected == row['expected'], (row['id'], 'expected outcome')
        assert golden == row['golden'], (row['id'], 'golden values')
        assert golden == row['evidence'][1]['golden'], (row['id'], 'integer evidence')
        assert row['evidence'][1]['input'] == row['input'], (row['id'], 'input association')
        assert row['evidence'][1]['wall'] == row['wall'], (row['id'], 'wall association')
    print('integer clamp golden rows', len(payload['entries']))


if __name__ == '__main__':
    check(sys.argv[1])
