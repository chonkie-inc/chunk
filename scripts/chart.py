#!/usr/bin/env python3
"""Generate benchmark comparison chart in Flexoki style."""

import matplotlib.pyplot as plt

# Flexoki colors (light theme)
FLEXOKI = {
    'black': '#100F0F',
    'base_950': '#1C1B1A',
    'base_900': '#282726',
    'base_800': '#403E3C',
    'base_700': '#575653',
    'base_600': '#6F6E69',
    'base_500': '#878580',
    'base_400': '#9F9D96',
    'base_300': '#B7B5AC',
    'base_200': '#CECDC3',
    'base_100': '#E6E4D9',
    'base_50': '#F2F0E5',
    'paper': '#FFFCF0',
    'green_400': '#879A39',
    'green_500': '#4D7534',
    'green_300': '#99AA5C',
    'green_200': '#C4D08F',
    'cyan_400': '#3AA99F',
    'orange_400': '#DA702C',
    'red_400': '#D14D41',
}

# Benchmark data - throughput in GB/s (4KB chunks, enwik8 100MB)
# Real measured values from quick_comparison.rs and python_comparison.py
chunkers = ['chunk', 'kiru', 'langchain', 'semchunk', 'llama-index', 'text-splitter']
speeds = [164, 4.5, 0.35, 0.013, 0.0035, 0.0017]  # GB/s

# Sort by speed (fastest first)
sorted_data = sorted(zip(chunkers, speeds), key=lambda x: -x[1])
chunkers, speeds = zip(*sorted_data)

fig, ax = plt.subplots(figsize=(10, 4.5))
fig.set_facecolor(FLEXOKI['paper'])
ax.set_facecolor(FLEXOKI['paper'])

# Colors: bright green for fastest, muted for others
colors = [FLEXOKI['green_400'] if i == 0 else FLEXOKI['base_300']
          for i in range(len(chunkers))]

# Horizontal bars
bars = ax.barh(range(len(chunkers)), speeds, color=colors, height=0.5)

# Y-axis labels
ax.set_yticks(range(len(chunkers)))
ax.set_yticklabels(chunkers, fontsize=14, color=FLEXOKI['base_800'], fontweight='medium')

# X-axis
ax.set_xlabel('GB/s', color=FLEXOKI['base_600'], fontsize=12)
ax.tick_params(axis='x', colors=FLEXOKI['base_600'])

# Configure spines - show bottom and left only
ax.spines['top'].set_visible(False)
ax.spines['right'].set_visible(False)
ax.spines['bottom'].set_color(FLEXOKI['base_600'])
ax.spines['left'].set_color(FLEXOKI['base_600'])

# Add slowdown labels for others
for i, (name, speed) in enumerate(zip(chunkers[1:], speeds[1:]), 1):
    slowdown = speeds[0] / speed
    ax.text(speed + speeds[0] * 0.02, i, f'{slowdown:.0f}x slower',
            va='center', color=FLEXOKI['base_600'], fontsize=11)

# Title
ax.set_title('Chunking speed', color=FLEXOKI['black'], fontsize=18,
             fontweight='bold', loc='center', pad=20)

# Subtitle
fig.text(0.95, 0.02, 'Speed in GB/s, enwik8 100MB, 4KB chunks, Apple M3',
         fontsize=9, color=FLEXOKI['base_500'], style='italic', ha='right')

# No grid lines
ax.xaxis.grid(False)
ax.yaxis.grid(False)

# Invert y-axis so fastest is on top
ax.invert_yaxis()

plt.tight_layout()
plt.subplots_adjust(bottom=0.18)
plt.savefig('benchmark.png', dpi=150, facecolor=FLEXOKI['paper'])
plt.savefig('benchmark.svg', facecolor=FLEXOKI['paper'])
print("Saved benchmark.png and benchmark.svg")
