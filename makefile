ifeq ($(OS),Windows_NT)
	NAME := beatrijs.exe
else
	NAME := beatrijs
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)
