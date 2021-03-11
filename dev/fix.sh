sed 's|\.js\$/, |\.js/, |' < target/wasm.js > target/wasm.js.new && mv target/wasm.js.new target/wasm.js
