set -e
set -x
if [[ -n $(git status -s | grep -v '??' | grep -v deploy_test.sh | grep -v bevy-atari-antic) ]]; then
  echo git not clean, qutting
  exit 1
fi

DEST=test.tmp
cargo make build-webgl -p release
mkdir -p $DEST/target
mkdir -p $DEST/pokey

cp -v web/index.html $DEST
cp -v -a web/wasm $DEST/wasm
cp -v web/pokey/pokey.js $DEST/pokey
cp -v -a web/js $DEST

#git checkout web && cp -av test.tmp/* test/
