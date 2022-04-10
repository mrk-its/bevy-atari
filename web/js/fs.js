export async function asyncify(func, ...args) {
    return new Promise((resolve, reject) => {
        function cb(err, ...args) {
            if (err) {
                reject(err)
            } else {
                resolve.apply(undefined, args)
            }
        }
        let func_args = [...args, cb]
        func.apply(undefined, func_args);
    });
}

export async function mkdirs(path) {
    let dirs = path.split("/").slice(0, -1)
    for (var i = 0; i < dirs.length; i++) {
        let dir = dirs.slice(0, i + 1).join("/");
        if (dir) {
            try {
                await asyncify(fs.mkdir, dir);
            } catch (err) {
                if (err.code == 'EEXIST') {
                    continue;
                }
                throw err;
            }
            console.log(`created dir ${dir}`);
        }
    }
}

export async function readDir(path) {
    return await asyncify(fs.readdir, path);
}

export async function readFile(path) {
    return await asyncify(fs.readFile, path);
}


export async function writeFile(path, buffer) {
    await asyncify(fs.writeFile, path, new _fs.Buffer(buffer));
}

export async function rm(path) {
    return await asyncify(fs.unlink, path)
}

export async function rmdir(path) {
    return await asyncify(fs.rmdir, path)
}

let _fs = {}
let fs;

export async function initFilesystem(backend) {
    BrowserFS.install(_fs);
    let ifbfs = await asyncify  (BrowserFS.FileSystem[backend].Create, {});
    console.info(`BrowserFS configured with IndexedDB ${backend}`);
    BrowserFS.initialize(ifbfs);
    fs = _fs.require("fs");
}
