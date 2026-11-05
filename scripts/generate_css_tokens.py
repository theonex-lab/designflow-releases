#!/usr/bin/env python3
"""Generate CSS variables from the canonical DesignFlow token JSON, or check for drift."""
from __future__ import annotations
import argparse
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
tokens = json.loads((root / 'assets/design-tokens.json').read_text(encoding='utf-8'))
variables = {
    'primary': tokens['color']['primary'],
    'primary-hover': tokens['color']['primaryHover'],
    'primary-subtle': tokens['color']['primarySubtle'],
    'text': tokens['color']['text'],
    'muted': tokens['color']['muted'],
    'surface': tokens['color']['surface'],
    'bg': tokens['color']['canvas'],
    'border': tokens['color']['border'],
    'danger': tokens['color']['danger'],
    'success': tokens['color']['success'],
    'focus': tokens['color']['focusRing'],
    'sidebar': f"{tokens['layoutPx']['sidebar']}px",
    'topbar': f"{tokens['layoutPx']['topbar']}px",
    'radius-sm': f"{tokens['radiusPx']['small']}px",
    'radius-md': f"{tokens['radiusPx']['medium']}px",
    'radius-lg': f"{tokens['radiusPx']['large']}px",
}
body = '/* GENERATED from assets/design-tokens.json — do not edit directly. */\n:root {\n'
body += ''.join(f'  --df-{name}: {value};\n' for name,value in variables.items())
body += '}\n'
output = root/'assets/ui/tokens.generated.css'
parser=argparse.ArgumentParser()
parser.add_argument('--check', action='store_true', help='Fail if generated file is stale')
args=parser.parse_args()
if args.check:
    assert output.exists(), f'missing generated file: {output}'
    assert output.read_text(encoding='utf-8')==body, 'Token drift detected; run python3 scripts/generate_css_tokens.py'
    print('OK: CSS variables match design-tokens.json')
else:
    output.write_text(body, encoding='utf-8')
    print(f'Generated {output.relative_to(root)}')
