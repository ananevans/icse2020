#/bin/bash

CRT_DIR=`pwd`
PROJECT_HOME="$HOME/work/unsafe_study/"

CRATES_DIR=/tmp/unsafe_analysis/crates.io-downloads/
COMPILER_OUTPUT_DIR=/tmp/unsafe_analysis/compiler_output

rm -rf $COMPILER_OUTPUT_DIR
mkdir $COMPILER_OUTPUT_DIR

rustup override set nightly-2018-08-29

export RUSTFLAGS="--extern unsafe_analysis=$PROJECT_HOME/unsafe-analysis/target/debug/libunsafe_analysis.so -Z extra-plugins=unsafe_analysis --emit mir"
export RUST_BACKTRACE=1
export RUST_LOG=error

cd $CRATES_DIR
for d in $(ls $CRATES_DIR)
do
	echo "Compiling $d"
	cd $d
	cargo build &> "$COMPILER_OUTPUT_DIR/$crate"
	cd ..
done

cd $CRT_DIR

