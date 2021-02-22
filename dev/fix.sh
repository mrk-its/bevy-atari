sed 's|\.js\$/, |\.js/, |' < wasm.js > wasm.js.new && mv wasm.js.new wasm.js
