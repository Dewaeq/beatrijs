#!/usr/bin/python3

import sys
import os

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
# os.system("git clone https://github.com/dewaeq/beatrijs")
# # os.system("git checkout master")
# os.system("cargo build --release")
# os.system("mv target/release/beatrijs beatrijs-master")


os.system("""
cutechess-cli \
-engine cmd=tmp/beatrijs-new name=new \
-engine cmd=tmp/beatrijs-master name=master \
-each restart=on tc=inf/10+0.1 book='/home/dewaeq/Downloads/baronbook30/baron30.bin' bookdepth=4 proto=uci option.hash=256 \
-games 2 -rounds 2500 -repeat 2 -maxmoves 200 \
-sprt elo0=0 elo1=10 alpha=0.05 beta=0.05 \
-concurrency 6 \
-ratinginterval 10 \
-recover \
-pgnout 'tmp/sprt.pgn'
""")
