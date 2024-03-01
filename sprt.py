#!/usr/bin/python3

import os
import argparse

parser = argparse.ArgumentParser(prog="Beatrijs SPRT test suite")
parser.add_argument("--hash", type=int, help="Table size in mb")
parser.add_argument("--threads", type=int, default=6, help="number of threads cutechess-cli can use")

args = parser.parse_args()
book_path = ""
engines = ["", ""]

# Compile current build
os.system("cargo build --release")

if os.name == "nt":
    os.system("rmdir /s /q tmp")
    os.system("mkdir tmp")
    engines = ["tmp/beatrijs-new.exe", "tmp/beatrijs-master.exe"]
else:
    os.system("rm -rf tmp")
    os.system("mkdir tmp")
    engines = ["tmp/beatrijs-new", "tmp/beatrijs-master"]

# Complile master build
os.system("cd tmp &&"
"git clone https://github.com/dewaeq/beatrijs &&"
"cd beatrijs &&"
"cargo build --release")

if os.name == "nt":
    book_path = 'D:/Quinten/Downloads/openings/baronbook30/baron30.bin'

    os.system("copy target\\release\\beatrijs.exe tmp\\beatrijs-new.exe")
    os.system("copy tmp\\beatrijs\\target\\release\\beatrijs.exe tmp\\beatrijs-master.exe")
else:
    book_path = '/home/dewaeq/Downloads/baronbook30/baron30.bin'

    os.system("cp target/release/beatrijs tmp/beatrijs-new")
    os.system("cp tmp/target/release/beatrijs tmp/beatrijs-master")


os.system(f"""
cutechess-cli \
-engine cmd=tmp/beatrijs-new name=new \
-engine cmd=tmp/beatrijs-master name=master \
-each restart=on tc=inf/8+0.08 book={book_path} \
bookdepth=4 proto=uci {f"option.Hash={args.hash}" if args.hash else ""} \
-games 2 -rounds 2500 -repeat 2 -maxmoves 200 \
-sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
-concurrency {args.threads} \
-ratinginterval 10 \
-recover \
-pgnout 'tmp/sprt.pgn'
""")
