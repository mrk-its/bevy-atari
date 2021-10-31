set -e
set -x
if [[ -n $(git status -s | grep -v '??' | grep -v bevy-atari-antic) ]]; then
  echo git not clean, qutting
  exit 1
fi

DEST=test.tmp
cargo make build-web
mkdir -p $DEST/target
mkdir -p $DEST/pokey

cp -v index.html $DEST
mv -v target/wasm_bg.wasm target/wasm.js $DEST/target
cp -v pokey/pokey.js $DEST/pokey
cp -v -a js $DEST

git checkout web && cp -av test.tmp/* test/
