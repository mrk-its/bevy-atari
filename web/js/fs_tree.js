import { readFile, writeFile, readDir, rm, rmdir } from './fs.js'

async function loadPath(node) {
    let root_path = node.getPath()
    let items = await readDir(root_path);
    let out = [];
    for (var name of items) {
        let path = `${root_path}/${name}`
        let folder = await readDir(path).then(() => true, () => false);
        out.push({
            title: name,
            folder,
            lazy: folder,
        })
    };
    return out;
}

export async function treeShowPath(key, path) {
    if (key == "basic" || key == "osrom") {
        return;
    }
    var tree = $.ui.fancytree.getTree("#tree");
    var node = tree.getFirstChild();
    await node.setExpanded(true);
    let parts = path.split("/").filter(x => x.length)
    for (var part of parts) {
        node = node.children.filter(c => c.title == part)[0]
        if (!node)
            break;
        if (!node.lazy) {
            node.setActive(true);
            break;
        }
        await node.setExpanded(true);
        node.setActive();
    }
}

export function treeInit() {
    $("input.file-reader").change(async function (e) {
        var file = e.target.files[0];
        let path = $(e.target).attr("data-path");
        let filename = e.target.filename;
        if (!file || !path) return;
        let buffer = await file.arrayBuffer();
        console.info(buffer, path, file, file.name);
        if (confirm(`replace contents of ${path}?`)) {
            writeFile(path, buffer);
        }
        $(e.target).val("").attr("data-path", null);
    })

    $("#tree").fancytree({
        extensions: ["table"],
        toggleEffect: false,
        renderColumns: function (event, data) {
            var node = data.node,
                $tdList = $(node.tr).find(">td");

            // (index #0 is rendered by fancytree by adding the checkbox)

            let save = $("<a class='save' href='#'>save</a>").click(async function (e) {
                e.preventDefault();
                let data = await readFile(node.getPath());
                let blob = new Blob([data])
                const a = document.createElement('a');
                document.body.appendChild(a);
                const url = window.URL.createObjectURL(blob);
                a.href = url;
                a.download = node.title;
                a.click();
            })
            let load = $("<a class='load' href='#'>load</a>").click(async function (e) {
                e.preventDefault();
                $('input.file-reader').attr("data-path", node.getPath()).click()
            })
            let del = $("<a class='del' href='#'>del</a>").click(async function (e) {
                e.preventDefault();
                if (!confirm(`delete ${node.title}?`)) return;
                if(node.folder) {
                    await rmdir(node.getPath());
                } else {
                    await rm(node.getPath());
                }
            })

            $tdList.eq(1).append(save);
            $tdList.eq(1).append(" ");
            $tdList.eq(1).append(load);
            $tdList.eq(1).append(" ");
            $tdList.eq(1).append(del);
        },
        source: [
            { title: "/", key: "root", folder: true, lazy: true },
        ],
        dblclick: function (event, data) {
            if (data.node.folder) return;
            window.location.hash = "#fs:" + data.node.getPath();
        },
        lazyLoad: function (event, data) {
            data.result = loadPath(data.node);
        },
    })
}