set -e
set -x

BUILD_DIR=./build

rm -fr $BUILD_DIR
mkdir -p $BUILD_DIR/target
mkdir -p $BUILD_DIR/pokey

cp -v web/index.html $BUILD_DIR
cp -v -a web/wasm $BUILD_DIR/wasm
cp -v web/pokey/pokey.js $BUILD_DIR/pokey
cp -v -a web/js $BUILD_DIR
