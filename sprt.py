#!/usr/bin/python3

import os
import argparse

parser = argparse.ArgumentParser(prog="Beatrijs SPRT test suite")
parser.add_argument("--hash", type=int, default=128, help="Table size in mb")
parser.add_argument("--threads", type=int, default=6,
                    help="number of threads cutechess-cli can use")
parser.add_argument("--branch", type=str, default="master",
                    help="git branch to test against")

args = parser.parse_args()
book_path = "./book.bin"

# Compile current build
os.system("cargo build --release")

if os.name == "nt":
    os.system("rmdir /s /q tmp")
    os.system("mkdir tmp")
else:
    os.system("rm -rf tmp")
    os.system("mkdir tmp")

# Complile master build
os.system("cd tmp &&"
          f"git clone -b {args.branch} https://github.com/dewaeq/beatrijs &&"
          "cd beatrijs &&"
          "cargo build --release")

if os.name == "nt":
    os.system("copy target\\release\\beatrijs.exe tmp\\beatrijs-new.exe")
    os.system(
        "copy tmp\\beatrijs\\target\\release\\beatrijs.exe tmp\\beatrijs-old.exe")
else:
    os.system("cp target/release/beatrijs tmp/beatrijs-new")
    os.system("cp tmp/target/release/beatrijs tmp/beatrijs-old")

os.system(f"""
cutechess-cli \
-engine cmd=tmp/beatrijs-new name=new \
-engine cmd=tmp/beatrijs-old name=old \
-each restart=on tc=inf/8+0.08 book={book_path} \
bookdepth=4 proto=uci option.Hash={args.hash} \
-games 2 -rounds 2500 -repeat 2 -maxmoves 200 \
-sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
-concurrency {args.threads} \
-draw movenumber=40 movecount=20 score=10 \
-resign movecount=15 score=600 \
-ratinginterval 10 \
-recover \
-pgnout tmp/sprt.pgn
""")
