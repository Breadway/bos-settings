#!/usr/bin/env python3
"""Compare prototype screenshots with the shipping app, page by page.

Both directories hold `<view>-<width>.png`. A pixel counts as different when
its luma differs by more than --threshold, and a page fails when that is more
than --limit of its pixels.

    scripts/pixel-diff.py /tmp/opencode/ref3 /tmp/opencode/shot3
    scripts/pixel-diff.py refs shots --widths 1920,960,860 --diffs /tmp/diffs

Exit status is 1 when a page is over the limit or a screenshot is missing.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageChops


def parse_widths(value: str) -> list[int]:
    return [int(part) for part in value.split(",") if part.strip()]


def compare(ref_path: Path, shot_path: Path, threshold: int) -> tuple[float, int, tuple[int, int], Image.Image]:
    ref = Image.open(ref_path).convert("L")
    shot = Image.open(shot_path).convert("L")
    if ref.size != shot.size:
        raise ValueError(f"size {ref.size} vs {shot.size}")
    diff = ImageChops.difference(ref, shot).point(lambda value: 255 if value > threshold else 0)
    changed = diff.histogram()[255]
    fraction = changed / (ref.width * ref.height)
    return fraction, changed, ref.size, diff


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("refs", type=Path, help="shipping screenshots")
    parser.add_argument("shots", type=Path, help="prototype screenshots")
    parser.add_argument("--widths", default="1920,960,860")
    parser.add_argument("--threshold", type=int, default=40, help="luma difference that counts")
    parser.add_argument("--limit", type=float, default=0.01, help="failing fraction, 0.01 is 1%%")
    parser.add_argument("--diffs", type=Path, help="write difference masks here")
    args = parser.parse_args()

    widths = parse_widths(args.widths)
    views = sorted(
        {
            path.name.rsplit("-", 1)[0]
            for path in args.shots.glob("*.png")
            if path.name.rsplit("-", 1)[-1].removesuffix(".png").isdigit()
        }
    )
    if not views:
        print(f"no <view>-<width>.png files in {args.shots}", file=sys.stderr)
        return 1

    failed = False
    results = []
    for view in views:
        for width in widths:
            name = f"{view}-{width}.png"
            ref_path = args.refs / name
            shot_path = args.shots / name
            if not ref_path.is_file() or not shot_path.is_file():
                missing = ref_path if not ref_path.is_file() else shot_path
                print(f"{view:12} {width:5} MISSING {missing}", file=sys.stderr)
                failed = True
                continue
            try:
                fraction, changed, size, diff = compare(ref_path, shot_path, args.threshold)
            except ValueError as err:
                print(f"{view:12} {width:5} SIZE    {err}", file=sys.stderr)
                failed = True
                continue
            results.append((fraction, view, width))
            flag = "FAIL" if fraction > args.limit else "ok"
            print(f"{view:12} {width:5} {fraction * 100:6.2f}% {changed:8d}px {size[0]}x{size[1]} {flag}")
            if args.diffs:
                args.diffs.mkdir(parents=True, exist_ok=True)
                diff.save(args.diffs / name)
            if fraction > args.limit:
                failed = True

    if results:
        worst = max(results)
        print(f"\nworst: {worst[1]} at {worst[2]}px, {worst[0] * 100:.2f}% over the {args.threshold} luma threshold")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
