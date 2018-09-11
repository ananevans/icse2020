#/bin/bash

CRT_DIR=`pwd`
PROJECT_HOME="$HOME/work/unsafe_study/"

CRATES_DIR=/tmp/unsafe_analysis/crates.io-downloads/
COMPILER_OUTPUT_DIR=/tmp/unsafe_analysis/compiler_output

rm -rf $COMPILER_OUTPUT_DIR
mkdir $COMPILER_OUTPUT_DIR

NIGHTLY=nightly-2018-09-08

export RUST_BACKTRACE=1
export RUST_LOG=error

cd ../unsafe-analysis
cargo +$NIGHTLY build

export RUSTFLAGS="--extern unsafe_analysis=$PROJECT_HOME/unsafe-analysis/target/debug/libunsafe_analysis.so -Z extra-plugins=unsafe_analysis --emit mir"

cd $CRATES_DIR
for d in $(ls $CRATES_DIR)
do
	echo "Compiling $d Output $COMPILER_OUTPUT_DIR/${d} Error $COMPILER_OUTPUT_DIR/${d}_error"
	cd $d
	cargo +$NIGHTLY clean
	cargo +$NIGHTLY build #> "$COMPILER_OUTPUT_DIR/${d}" 2>"$COMPILER_OUTPUT_DIR/${d}_error"
	cd ..
done

cd $CRT_DIR

