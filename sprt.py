#!/usr/bin/python3

import sys
import os
import argparse

parser = argparse.ArgumentParser(prog="Beatrijs SPRT test suite")
parser.add_argument("--hash", type=int, help="Table size in mb")
parser.add_argument("--threads", type=int, default=6, help="number of threads cutechess-cli can use")

args = parser.parse_args()

os.system("rm -rf tmp")
os.system("mkdir tmp")

# Compile current build
os.system("cargo build --release")
os.system("cp target/release/beatrijs tmp/beatrijs-new")

# Complile master build
os.system("""cd tmp &&
          git clone https://github.com/dewaeq/beatrijs &&
          cd beatrijs &&
          cargo build --release &&
          cp target/release/beatrijs ../beatrijs-master
          """)

os.system(f"""
cutechess-cli \
-engine cmd=tmp/beatrijs-new name=new \
-engine cmd=tmp/beatrijs-master name=master \
-each restart=on tc=inf/8+0.08 book='/home/dewaeq/Downloads/baronbook30/baron30.bin' \
bookdepth=4 proto=uci {f"option.Hash={args.hash}" if args.hash else ""} \
-games 2 -rounds 2500 -repeat 2 -maxmoves 200 \
-sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
-concurrency {args.threads} \
-ratinginterval 10 \
-recover \
-pgnout 'tmp/sprt.pgn'
""")
