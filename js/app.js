import init, { set_joystick, set_consol, set_binary_data, cmd, reset, set_state } from '../target/wasm.js'

import { SAPWriter } from './sap_writer.js'

const k_file_header = [150, 2, 96, 17, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 7, 20, 7, 76, 20, 7, 116, 137, 0, 0, 169, 70, 141, 198, 2, 208, 254, 160, 0, 169, 107, 145, 88, 32, 217, 7, 176, 238, 32, 196, 7, 173, 122, 8, 13, 118, 8, 208, 227, 165, 128, 141, 224, 2, 165, 129, 141, 225, 2, 169, 0, 141, 226, 2, 141, 227, 2, 32, 235, 7, 176, 204, 160, 0, 145, 128, 165, 128, 197, 130, 208, 6, 165, 129, 197, 131, 240, 8, 230, 128, 208, 2, 230, 129, 208, 227, 173, 118, 8, 208, 175, 173, 226, 2, 141, 112, 7, 13, 227, 2, 240, 14, 173, 227, 2, 141, 113, 7, 32, 255, 255, 173, 122, 8, 208, 19, 169, 0, 141, 226, 2, 141, 227, 2, 32, 174, 7, 173, 122, 8, 208, 3, 76, 60, 7, 169, 0, 133, 128, 133, 129, 133, 130, 133, 131, 173, 224, 2, 133, 10, 133, 12, 173, 225, 2, 133, 11, 133, 13, 169, 1, 133, 9, 169, 0, 141, 68, 2, 108, 224, 2, 32, 235, 7, 133, 128, 32, 235, 7, 133, 129, 165, 128, 201, 255, 208, 16, 165, 129, 201, 255, 208, 10, 32, 235, 7, 133, 128, 32, 235, 7, 133, 129, 32, 235, 7, 133, 130, 32, 235, 7, 133, 131, 96, 32, 235, 7, 201, 255, 208, 9, 32, 235, 7, 201, 255, 208, 2, 24, 96, 56, 96, 173, 9, 7, 13, 10, 7, 13, 11, 7, 240, 121, 172, 121, 8, 16, 80, 238, 119, 8, 208, 3, 238, 120, 8, 169, 49, 141, 0, 3, 169, 1, 141, 1, 3, 169, 82, 141, 2, 3, 169, 64, 141, 3, 3, 169, 128, 141, 4, 3, 169, 8, 141, 5, 3, 169, 31, 141, 6, 3, 169, 128, 141, 8, 3, 169, 0, 141, 9, 3, 173, 119, 8, 141, 10, 3, 173, 120, 8, 141, 11, 3, 32, 89, 228, 173, 3, 3, 201, 2, 176, 34, 160, 0, 140, 121, 8, 185, 128, 8, 170, 173, 9, 7, 208, 11, 173, 10, 7, 208, 3, 206, 11, 7, 206, 10, 7, 206, 9, 7, 238, 121, 8, 138, 24, 96, 160, 1, 140, 118, 8, 56, 96, 160, 1, 140, 122, 8, 56, 96, 0, 3, 0, 128, 0, 0, 0, 0, 0, 0];

const BINARY_KEYS = ['disk_1', 'osrom', 'basic', 'car'];
const DEFAULT_OSROM_URL = "https://atarionline.pl/utils/9.%20ROM-y/Systemy%20operacyjne/Atari%20OS%20v2%2083.10.05.rom"

var sap_writer = null;
var pokeyNode;
export var audio_context;

var atr_images = {}

class Binary {
  constructor(data, file_name, url, file_handle) {
    this.emulator_loadable = true
    this.data = data
    this.file_handle = file_handle
    this.file_name = file_name
    this.url = url
  }
}

class ATR extends Binary {
  constructor(data, file_name, url, file_handle) {

    if(data[0] == 255 && data[1] == 255) {
      data = xex2atr(data);
      file_handle = null;
      file_name = "[auto-k-file].atr";
    }

    super(data, file_name, url, file_handle)
    this.emulator_loadable = false
    if (data[0] != 0x96 || data[1] != 0x2) throw "bad atr magic!";
    this.sector_size = data[4] + 256 * data[5];
    console.info("sector size: ", this.sector_size);
    if (this.sector_size != 128 && this.sector_size != 256) {
      throw `invalid sector size: ${this.sector_size}`
    }
    this.version = 0
  }
  get_sector(sector) {
    if(sector <= 3) {
      return this.data.subarray(16 + (sector - 1) * 128, 16 + sector * 128)
    }
    return this.data.subarray(16 + 3 * 128 + (sector - 4) * this.sector_size, 16 + 3 * 128 + (sector - 3) * this.sector_size);
  }
  put_sector(sector, data) {
    this.get_sector(sector).set(data);
    this.version += 1;
    if (this.on_modify) this.on_modify()
  }
}


class GUI {
  constructor() {
    this.binaries = {}
  }

  createSlot(id, name, klass) {
    let container = $('<div>', { "id": id })
    $("<span>", { "class": "label" }).text(name).appendTo(container)
    $("<span>", { "class": "name" }).appendTo(container)
    let actions = $('<div>').addClass("actions")

    let _this = this

    if (window.showOpenFilePicker) {
      let save_action = $("<a>", { "class": "save" }).attr("href", "#").text("save").appendTo(actions)
      let open_action = $("<a>", { "class": "open", "href": "#" }).text("open").appendTo(actions);
      open_action.click(async (e) => {
        e.preventDefault();
        let handle = (await window.showOpenFilePicker())[0];
        let file = await handle.getFile();
        let buffer = new Uint8Array(await file.arrayBuffer());
        _this.setBinary(id, new klass(buffer, file.name, null, handle));
      })

      save_action.click(async (e) => {
        e.preventDefault();
        let atr = _this.getBinary(id)
        if (!atr || !atr.file_handle) {
          if(!atr) {
            console.warn("no atr");
            return;
          }
          // console.warn("no handle")
          let options = { suggestedName: atr.file_name }
          let handle = await window.showSaveFilePicker(options);
          let file = await handle.getFile()
          atr.file_name = file.name
          atr.file_handle = handle
          atr.url = null;
          _this.setBinary(id, atr)  // update
        }
        let version = atr.version;
        let writable = await atr.file_handle.createWritable();
        writable.write(atr.data)
        writable.close()
        if (atr.version == version) {
          $(`#${id}`).removeClass("modified")
        }
      });
    } else {
      let save_action = $("<a>", { "class": "save" }).attr("href", "#").text("save").appendTo(actions)
      let open_action = $("<input type='file'>").addClass("custom-file-input").appendTo(actions)
      open_action.change(async e => {
        var file = e.target.files[0];
        if (file) {
          let buffer = new Uint8Array(await file.arrayBuffer());
          _this.setBinary(id, new klass(buffer, file.name, null, null));
        }
      })
      save_action.click(async e => {
        let cont = $(`#${id}`)
        let atr = _this.getBinary(id)
        let blob_url = URL.createObjectURL(new Blob([atr.data]))
        save_action.attr("href", blob_url).attr("download", cont.find("span.name").text())
      })
    }

    let eject = $("<a>", { "class": "eject", "href": "#" }).text("eject").appendTo(actions);


    eject.click((e) => {
      e.preventDefault();
      let binary = _this.ejectBinary(id);
      if(binary && binary.url)
        set_fragment(parse_fragment().filter(k => k[1] != binary.url))
    })
    container.append(actions)
    return container;
  }

  createInterface() {
    $('div.files')
      .append(this.createSlot("osrom", "OS", Binary))
      .append(this.createSlot("basic", "Basic", Binary))
      .append(this.createSlot("car", "Cartridge", Binary))
      .append(this.createSlot("disk_1", "Disk 1", ATR))
      .append(this.createSlot("disk_2", "Disk 2", ATR));
  }

  setBinary(id, binary) {
    this.binaries[id] = binary
    if(binary.emulator_loadable)
      set_binary_data(id, binary.file_name, binary.data)
    let cont = $(`#${id}`)
    binary.on_modify = () => {
      cont.addClass("modified")
    }
    cont.addClass("loaded")
    cont.find("span.name").text(binary.file_name || 'no-name')

  }
  ejectBinary(id) {
    let binary = this.binaries[id]
    delete this.binaries[id]
    let cont = $(`#${id}`)
    cont.removeClass("loaded")
    cont.find("span.name").text("")
    if(binary.emulator_loadable) set_binary_data(id, "", [])
    return binary
  }
  getBinary(id) {
    return this.binaries[id]
  }
}

function _drive_id(drive, unit) {
  return `disk_${drive + unit - 1 - 48}`
}

function sio_get_status(drive, unit, data) {
  let atr = window.gui.getBinary(_drive_id(drive, unit));
  var status = 0xff;
  if(atr) {
    data[0] = atr.sector_size == 256 ? 0x20 : 0;
    data[1] = 0;
    data[2] = 0;
    data[3] = 0;
    status = 0x01;
  } else {
    data[0] = 255;
    data[1] = 255;
    data[2] = 255;
    data[3] = 255;
  }
  console.log(`sio_get_status drive: ${drive}, unit: ${unit}, status: ${status}`);
  return status;
}

function sio_get_sector(drive, unit, sector, data) {
  let atr = window.gui.getBinary(_drive_id(drive, unit))
  var status;
  if (atr) {
    var read_data = atr.get_sector(sector);
    if(read_data.length == data.length) {
      data.set(read_data)
      status = 0x01;
    } else {
      console.error(`read length: ${read_data.length}, buffer length: ${data.length}`)
      status = 0xff;
    }
  } else {
    status = 0xff;
  }
  console.log(`sio_get_sector drive: ${drive}, unit: ${unit}, sector: ${sector}, len: ${data.length}, status: ${status}`);
  return status;
}

function sio_put_sector(drive, unit, sector, data) {
  let atr = window.gui.getBinary(_drive_id(drive, unit))
  var status;
  if (atr) {
    atr.put_sector(sector, data)
    status = 0x1;
  } else {
    status = 0xff;
  }

  console.log(`sio_put_sector drive: ${drive}, unit: ${unit}, sector: ${sector}, len: ${data.length}, status: ${status}`);
  return status;
}

export function rec_start_stop(event) {
  let button = event.target
  if (sap_writer == null) {
    let is_stereo = $("#sap-r-writer input.stereo").is(":checked")
    let trim = $("#sap-r-writer input.trim").is(":checked")
    console.log("trim:", trim)
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
  if(!pokeyNode) return;
  pokeyNode.port.postMessage(msg);
  if (sap_writer)
    sap_writer.handle_pokey_msg(msg)
}


function xex2atr(data) {
  let n_sectors = Math.floor((data.length + 127) / 128) + 3;
  let size = n_sectors * 128 / 16; // size in paragraphs;
  let size_h = Math.floor(size / 256);
  let size_l = size % 256;
  let atr_buf = new Uint8Array(n_sectors * 128 + 16);
  atr_buf.set(k_file_header, 0);
  atr_buf.set(data, k_file_header.length);
  atr_buf[2] = size_l;
  atr_buf[3] = size_h;
  atr_buf[25] = data.length % 256;
  atr_buf[26] = Math.floor(data.length / 256);
  return atr_buf;
}

function parse_part(part) {
  let m = part.match("^(\\w+)(@(\\d+))?=(.*)");
  return m && [m[1], m[4], m[3]] || [null, part, null]
}

function parse_fragment() {
  let hash = document.location.hash.substring(1)
  let sep = new RegExp('\\|\\||&&')
  return hash.split(sep).map(parse_part).filter( i => i[1] && i[1].length)
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

function set_binary(key, url, data, slot) {
  var filename = url_to_filename(url);
  let parts = filename.split(".")
  let ext = parts[parts.length - 1];
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
      let is_valid = (data[0] == 0x96 && data[1] == 0x02 && data[4] == 128 && data[5] == 0);
      if (is_valid) {
        key = "disk_1"
      } else {
        console.warn("unsupported ATR file");
        return;
      }
    } else if (ext == "xex") {
      let is_valid = (data[0] == 255 && data[1] == 255);
      if(is_valid) {
        key = "disk_1"
      } else {
        console.warn("invalid xex header");
        return;
      }
      // handled below
    } else {
      console.warn("unknown type of file", filename);
      return
    }
  }
  let klass = key.startsWith("disk_") ? ATR : Binary;
  window.gui.setBinary(key, new klass(data, filename, url));
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

function fetch_url(url) {
  if (is_absolute_url(url)) {
    url = "https://atari.ha.sed.pl/" + url;
  }
  return fetch(url).then(r => {
    let content_disposition = r.headers.get("Content-Disposition");
    // TODO use filename from content_disposition
    return r.arrayBuffer()
  }).then(function (data) {
    return new Uint8Array(data);
  })
}

function fetch_binary_data(key, url, slot) {
  console.log("fetch_binary_data", key, url, slot)
  return fetch_url(url).then(function (data) {
    let type = set_binary(key, url, data, slot);
    console.log("set_binary", key, url, "len:", data.length);
    return type;
  })
}

function delay(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export function on_hash_change() {
  reload_from_fragment();
}

async function reload_from_fragment() {
  console.log("calling set_state: idle");
  set_state("idle");
  await delay(100);
  let todo = [];
  for (let [key, url, slot] of parse_fragment()) {
    todo.push(fetch_binary_data(key, url, parseInt(slot)));
  };
  let result = await Promise.all(todo);
  let result_set = new Set(result);
  if (!result_set.has("osrom")) {
    await fetch_binary_data(null, DEFAULT_OSROM_URL);
    result_set.add("osrom");
  }
  let to_remove = BINARY_KEYS.filter(x => !result_set.has(x))
  for (let key of to_remove) {
    set_binary_data(key, "", []);
  }
  reset(true, true);
  console.log("calling set_state: running");
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
  console.log("initialized")
  var latencyHint = parseFloat(localStorage.latencyHint);
  if (!(latencyHint >= 0)) latencyHint = localStorage.latencyHint || "playback";
  console.log("latencyHint: ", latencyHint);
  let audio_context = new AudioContext({
    sampleRate: 48000,
    latencyHint: latencyHint,
  });
  console.log("sampleRate: ", audio_context.sampleRate);
  if(audio_context.audioWorklet) {
    await audio_context.audioWorklet.addModule('pokey/pokey.js')
    pokeyNode = new AudioWorkletNode(audio_context, 'POKEY', {
      outputChannelCount: [2],  // stereo
    })
    pokeyNode.connect(audio_context.destination)

    document.addEventListener(
      'visibilitychange',
      e => document.hidden ? window.audio_context.suspend() : window.audio_context.resume()
    );
  } else {
    console.warn("audio_context.audioWorklet is undefined (serving through http?)");
  }
  window.pokey_post_message = pokey_post_message
  window.audio_context = audio_context
  window.cmd = cmd

  window.sio_get_sector = sio_get_sector;
  window.sio_get_status = sio_get_status;
  window.sio_put_sector = sio_put_sector;

  try {
    await init()
  } catch (e) {
    !e.toString().match("This isn't actually an error") && console.error(e);
  }

  window.gui = new GUI()
  window.gui.createInterface();

  reload_from_fragment();
  auto_focus()

}
