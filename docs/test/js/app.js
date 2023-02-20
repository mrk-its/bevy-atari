import init, { set_binary_data, cmd, reset as _reset, set_state, set_resolution as _set_resolution, keystrokes} from '../wasm/wasm.js'

import { SAPWriter } from './sap_writer.js'
import { initFilesystem, mkdirs, readFile, writeFile, readDir, rm } from './fs.js'
import { treeInit, treeShowPath } from './fs_tree.js'

const BINARY_KEYS = ['disk_1', 'osrom', 'basic', 'car', 'xex'];
const DEFAULT_OSROM_URL = "https://atarionline.pl/utils/9.%20ROM-y/Systemy%20operacyjne/Atari%20OS%20v2%2083.10.05.rom"
const DEFAULT_BASIC_URL = "https://atarionline.pl/utils/9.%20ROM-y/Języki%20programowania/Atari%20BASIC/Atari%20Basic%20vB.rom"
var sap_writer = null;
var pokeyNode;
export var audio_context;

export const set_resolution = _set_resolution;
export const reset = _reset;

var atr_images = {}

const NO_PROXY_RE = /^data:|^https?:\/\/(localhost|127.\d+.\d+.\d+|atari.ha.sed.pl)/
const FORCE_PROXY_RE = /^http:|^https:\/\/(atarionline.pl|atariwiki.org)\//
const HTTP_RE = /^https?:\/\//

function cors_fetch_url(url) {
  return fetch('https://atari.ha.sed.pl/' + url)
}

function fetch_url(url) {
  if(NO_PROXY_RE.test(url) || !HTTP_RE.test(url)) {
    return fetch(url)
  } else if(FORCE_PROXY_RE.test(url)) {
    return cors_fetch_url(url)
  } else {
    return fetch(url).catch(e => cors_fetch_url(url))
  }
}

export function rec_start_stop(event) {
  let button = event.target
  if (sap_writer == null) {
    let is_stereo = $("#sap-r-writer input.stereo").is(":checked")
    let trim = $("#sap-r-writer input.trim").is(":checked")
    sap_writer = new SAPWriter(is_stereo, trim)
    button.innerText = 'Stop';
    document.getElementById("download_url").innerHTML = ''
  } else {
    let writer = sap_writer
    sap_writer = null
    button.innerText = 'Rec'
    let writer_div = $("#sap-r-writer")
    let author = writer_div.find("input[name=author]").val()
    let name = writer_div.find("input[name=name]").val()

    var parts = []
    var sap_headers = []
    if (name) {
      parts.push(name)
      sap_headers.push(`NAME "${name}"`)
    }
    if (author) {
      parts.push(author)
      sap_headers.push(`AUTHOR "${author}"`)
    }
    let fn = parts.join("_") || "file"

    let sap_data = writer.get_sap(sap_headers)
    var blobUrl = URL.createObjectURL(new Blob([sap_data.buffer]))


    var link = document.getElementById("download_url")
    link.href = blobUrl
    link.download = `${fn}.sap`
    link.innerHTML = "download"
  }
}

$(window).bind("sap_writer", event => {
  let data = event.originalEvent.data;
  $("#sap-r-writer .time-info").text(`${data.duration} / ${(data.data_size / 1024).toFixed(1)} kB`)
});

function pokey_post_message(msg) {
  if (!pokeyNode) return;
  pokeyNode.port.postMessage(msg);
  if (sap_writer)
    sap_writer.handle_pokey_msg(msg)
}

function parse_part(part) {
  let m = part.match("^(\\w+)(@(\\d+))?=(.*)");
  return m && [m[1], m[4], m[3]] || [null, part, null]
}

function parse_fragment() {
  let hash = document.location.hash.substring(1)
  let sep = new RegExp('\\|\\||&&')
  return hash.split(sep).map(parse_part).filter(i => i[1] && i[1].length)
}

function set_fragment(parts) {
  document.location.hash = parts.map(k => k[1]).join("||");
}

export function eject(event) {
  event.preventDefault();
  let node = event.target.parentNode.parentNode;
  let key = node.attributes.id.value;
  let url = node.attributes["data-url"].value;
}

function set_binary(key, url, path, data, slot) {
  var filename = path
  let parts = filename.split(".")
  let ext = parts[parts.length - 1].toLowerCase()
  if (!key) {
    // guess type of binary
    if (ext == "rom" || ext == "bin") {
      if (data.length == 0x4000) {
        key = "osrom"
      } else if (data.length == 0x2000) {
        key = "basic"
      } else {
        console.warn("invalid length of rom file", data.length);
        return;
      }
    } else if (ext == "state") {
      key = "state"
    } else if (ext == "car") {
      key = "car"
    } else if (ext == "atr") {
      key = "disk_1"
    } else if (ext == "xex") {
      key = "xex"
      // handled below
    } else {
      console.warn("unknown type of file", filename);
      return
    }
  }
  set_binary_data(key, filename, data)
  return key;
}

function is_absolute_url(url) {
  return url && (url.startsWith("http://") || url.startsWith("https://"));
}

function url_to_filename(url) {
  var path;
  if (is_absolute_url(url)) {
    let url_obj = new URL(url);
    path = decodeURIComponent(url_obj.pathname);
  } else {
    path = url;
  }
  let fn = path.split("/");
  fn = fn[fn.length - 1];
  return fn;
}

function fetch_buffer(url) {
  return fetch_url(url).then(r => {
    let content_disposition = r.headers.get("Content-Disposition");
    console.log("Content-Disposition:", content_disposition);
    // TODO use filename from content_disposition
    return r.arrayBuffer()
  }).then(function (data) {
    return new Uint8Array(data);
  })
}

async function hash(text) {
  let buf = await crypto.subtle.digest(
    'SHA-256',
    new TextEncoder("utf-8").encode(text)
  );
  return Array.from(new Uint8Array(buf)).slice(0, 16).map(i => ('0' + i.toString(16)).slice(-2)).join('')
}

async function url_to_path(url) {
  if(url.startsWith("data:")) {
    return null;
  }
  let url_obj = new URL(url);
  var path = url_obj.hostname + decodeURIComponent(url_obj.pathname);
  if (url_obj.search) {
    let h = await hash(url_obj.search);
    if (!path.endsWith("/")) path = path + "/";
    path = path + h;
  }
  return path
}

const VALID_URL_RE = /^data:|^fs:|^https?:\/\//

async function fetch_binary_data(key, url, slot) {
  if(!VALID_URL_RE.test(url)) return;

  console.log("fetch_binary_data", key, url, slot)
  let path = await url_to_path(url);
  var data;
  if(url.startsWith("fs:")) {
    data = await readFile(path)
    treeShowPath(key, path);
  } else if(url.startsWith("data:")) {
    data = await fetch_buffer(url);
  } else {
    try {
      if(document.location.hash.indexOf("no-cache")<0) {
        data = await readFile(path)
        treeShowPath(key, path);
      }
      console.info(`${path} read from cache`)
    } catch (err) {
      console.log(err);
    }
    if(!data) {
      data = await fetch_buffer(url);
      console.log(data);
      if(!url.startsWith("data:")) {
        try {
          await mkdirs(path);
          await writeFile(path, data.buffer);
          treeShowPath(key, path);
          console.log(`${path} written to cache`);
        } catch (err) {
          console.log("ERR:", err);
        }
      }
    }  
  }

  let type = set_binary(key, url, path, data, slot);
  console.log("set_binary", key, url, "len:", data.length);
  return type;
}

function delay(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export function on_hash_change() {
  reload_from_fragment();
}

async function reload_from_fragment() {
  set_state("idle");
  await delay(100);
  let todo = [];
  for (let [key, url, slot] of parse_fragment()) {
    if(!key || BINARY_KEYS.indexOf(key) >= 0) {
      todo.push(fetch_binary_data(key, url, parseInt(slot)));
    } else if(key == "keystrokes") {
      keystrokes(decodeURIComponent(url))
    }
  };
  let result = await Promise.all(todo);
  let result_set = new Set(result);
  if(result_set.has("xex") || result_set.has("disk_1")) {
    result_set.add("xex");
    result_set.add("disk_1");
  }
  if (!result_set.has("osrom")) {
    await fetch_binary_data("osrom", DEFAULT_OSROM_URL);
    result_set.add("osrom");
  }
  if (!result_set.has("basic")) {
    await fetch_binary_data("basic", DEFAULT_BASIC_URL);
    result_set.add("basic");
  }
  let to_remove = BINARY_KEYS.filter(x => !result_set.has(x))
  console.log("to_remove:", to_remove)
  for (let key of to_remove) {
    set_binary_data(key, "", []);
  }
  reset(true);
  set_state("running");
}

function auto_focus() {
  let canvas = document.getElementsByTagName("canvas");
  if (!canvas.length) {
    setTimeout(auto_focus, 100);
  } else {
    canvas[0].focus();
  }
}

export async function run() {
  await initFilesystem("IndexedDB");
  await mkdirs("/local/");
  treeInit();

  console.log("initialized")
  var latencyHint = parseFloat(localStorage.latencyHint);
  if (!(latencyHint >= 0)) latencyHint = localStorage.latencyHint || "playback";
  console.log("latencyHint: ", latencyHint);
  let audio_context = new AudioContext({
    sampleRate: 48000,
    latencyHint: latencyHint,
  });
  console.log("sampleRate: ", audio_context.sampleRate);
  if (audio_context.audioWorklet) {
    try {
      await audio_context.audioWorklet.addModule('pokey/pokey.js')
      pokeyNode = new AudioWorkletNode(audio_context, 'POKEY', {
        outputChannelCount: [2],  // stereo
      })
      pokeyNode.connect(audio_context.destination)

      document.addEventListener(
        'visibilitychange',
        e => document.hidden ? window.audio_context.suspend() : window.audio_context.resume()
      );
    } catch(err) {
      console.error("WebAudio:", err);
    }
  } else {
    console.warn("audio_context.audioWorklet is undefined (serving through http?)");
  }
  window.pokey_post_message = pokey_post_message
  window.audio_context = audio_context
  window.cmd = cmd


  window.attach = (id, path) => readFile(path).then(data => window.gui.setBinary(id, new ATR(data, path)));
  window.ls = path => readDir(path).then(r=> {console.info(r); return r});
  window.readFile = readFile
  window.writeFile = writeFile
  window.rm = path => rm(path).then(() => console.info("removed"));

  try {
    await init()
  } catch (e) {
    !e.toString().match("This isn't actually an error") && console.error(e);
  }

  $("body").on("dragover", false).on("drop", e => {
    let url = e.originalEvent.dataTransfer.getData('text/plain')
    if (url) window.location.hash = '#' + url;
    e.preventDefault();
  });

  reload_from_fragment();
  auto_focus()

}
