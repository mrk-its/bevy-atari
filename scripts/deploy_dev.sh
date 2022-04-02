git checkout web --recurse-submodules --

rm -fr docs/test

mv build_dir docs/test


git add docs/test

git commit -m "test release"
git push --set-upstream origin web
