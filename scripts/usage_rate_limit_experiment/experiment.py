"""Usage API rate limit experiment."""

import argparse
import asyncio
import sys


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Anthropic usage API rate limit experiment")
    p.add_argument("--enable-burst", action="store_true", help="Enable burst phase (risky)")
    p.add_argument("--resume", action="store_true", help="Resume from checkpoint")
    p.add_argument("--output-dir", default="experiment_output", help="Output directory")
    return p.parse_args()


async def run(args: argparse.Namespace) -> None:
    print(f"Output dir: {args.output_dir}")
    print("TODO: implement phases")


def main():
    args = parse_args()
    asyncio.run(run(args))


if __name__ == "__main__":
    main()
