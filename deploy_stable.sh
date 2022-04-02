set -e
set -x
#if [[ -n $(git status -s | grep -v '??' | grep -v deploy_test.sh | grep -v bevy-atari-antic) ]]; then
#  echo git not clean, qutting
#  exit 1
#fi

git checkout web && git reset --hard master || (echo "cannot reset web branch"; exit 1)

cargo make build-webgl-sha1 -p release
DEST=docs
rm -fr $DEST
mkdir -p $DEST/target
mkdir -p $DEST/pokey

cp -v web/index.html $DEST
cp -v -a web/wasm $DEST/wasm
cp -v web/pokey/pokey.js $DEST/pokey
cp -v -a web/js $DEST

rm -fr docs_test
cp -a docs docs_test && mv docs_test docs/test

git add docs
git config user.name github-actions
git config user.email github-actions@github.com
git commit -m "stable release"

git push -f