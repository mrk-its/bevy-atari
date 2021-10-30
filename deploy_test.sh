set -e
set -o pipefail

DEST=test.tmp
cargo make build-web
mkdir -p $DEST/target
mkdir -p $DEST/pokey

cp -v index.html $DEST
mv -v target/wasm_bg.wasm target/wasm.js $DEST/target
cp -v pokey/pokey.js $DEST/pokey
cp -v -a js $DEST

git checkout web && cp -av test.tmp/* test/
