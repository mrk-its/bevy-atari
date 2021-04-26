const EMPTY_POKEY_REGS = [0, 0, 0, 0, 0, 0, 0, 0, 0];

export class SAPPlayer {
    constructor() {
        this.headers = [];
        this.data = null;
        this.current_frame = 0;
        this.frame_cnt = 0;
        this.startTime = null;
        this.state = "stopped";
    }

    seek(pos) {
        this.current_frame = parseInt(pos);
        window.pokey_port.postMessage("clear_buffer");
        this.loadCurrentFrame();
        if(this.startTime != null) {
            this.startTime = null;
            this.play();
        }
    }

    _parse_headers(headers_data) {
        let decoder = new TextDecoder();
        let headers = decoder.decode(headers_data).split("\n")
        let headers_obj = {}
        for(let header of headers) {
            let key = header.split(" ", 1);
            headers_obj[key] = header.substring(key[0].length + 1).trim();       }
        return headers_obj
    }

    load(array_buffer) {
            let data = new Uint8Array(array_buffer);
        var ptr=0;
        while(ptr < 1024) {
            if(data[ptr] == 13 && data[ptr + 1] == 10 && data[ptr + 2] == 13 && data[ptr + 3] == 10) {
                this.headers = this._parse_headers(data.slice(0, ptr));
                console.log(this.headers);
                if(this.headers.TYPE != "R") {
                    console.warn(`TYPE: ${this.headers.TYPE} - only R type is supported`);
                    this.data = new Uint8Array();
                } else {
                    this.data = data.slice(ptr + 4);
                }
                var is_ntsc = typeof this.headers.NTSC != "undefined"
                var fastplay = parseInt(this.headers.FASTPLAY) || 0;
                if(fastplay) {
                    this.frame_interval = 1 / ((is_ntsc ? 262 * 60 : 312 * 50) / fastplay);
                } else if (is_ntsc) {
                    this.frame_interval = 1 / 60;
                } else {
                    this.frame_interval = 1 / 50;
                }
                this.frame_cnt = Math.floor(this.data.length / 9);
                this.current_frame = 0;
                this.sendEvent();
                let is_ok = this.data.length > 0;
                return is_ok;
            }
            ptr++;
        }
        console.warn("cannot locate data section");
        return false;
    }
    getPokeyRegs(index, ts) {
        let regs = Array.from(this.data.slice(index * 9, (index + 1) * 9));
        if(ts) regs.push(ts);
        return regs;
    }
    loadCurrentFrame() {
        let regs = this.data.slice(this.current_frame * 9, this.current_frame * 9 + 9);
        window.pokey_port.postMessage(regs);
        this.sendEvent(regs);
    }
    sendEvent(regs) {
        let event = new Event("sap_player");
        event.data = {
            current_frame: this.current_frame,
            frame_cnt: this.frame_cnt,
            pokey_regs: regs || null,
            state: this.state,
        }
        window.dispatchEvent(event);
    }
    tick() {
        this.fillBuffer()
    }
    fillBuffer() {
        if(!this.frame_cnt || this.state != "playing") {
            return;
        }
        let currentTime = this.getCurrentTime();
        while(this.startTime + this.current_frame * this.frame_interval < currentTime + 0.2) {
            let regs = this.getPokeyRegs(this.current_frame, this.startTime + this.current_frame * this.frame_interval);
            window.pokey_port.postMessage(regs);
            this.sendEvent(regs);
            this.current_frame = (this.current_frame + this.frame_cnt + 1) % this.frame_cnt;
            if(this.current_frame == 0) {
                this.startTime = currentTime;
                return;
            }
        }
    }
    getCurrentTime() {
        return window.audio_context.currentTime;
    }
    play() {
        let currentTime = this.getCurrentTime();
        if(this.startTime == null) {
            this.startTime = currentTime - this.current_frame * this.frame_interval;
        }
        this.state = "playing";
        this.fillBuffer();
    }
    pause() {
        this.state = "paused";
        this.interval = null;
        this.startTime = null;
        this.loadCurrentFrame();
    }
    stop() {
        this.state = "stopped";
        this.startTime = null;
        this.current_frame = 0;
        window.pokey_port.postMessage(EMPTY_POKEY_REGS);
        this.sendEvent(EMPTY_POKEY_REGS)
    }

    prev() {
        if (!this.data.length) return;
        this.pause();
        this.current_frame = (this.current_frame + this.frame_cnt - 1) % this.frame_cnt;
        this.loadCurrentFrame();
    }
    next() {
        if (!this.data.length) return;
        this.pause();
        this.current_frame = (this.current_frame + 1) % this.frame_cnt;
        this.loadCurrentFrame();
    }
}
