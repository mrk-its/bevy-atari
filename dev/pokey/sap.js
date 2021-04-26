export class SAPPlayer {
    constructor() {
        this.interval = null;
        this.headers = [];
        this.data = null;
        this.seek_widget = $('#player .seek')[0];
        this.position_info = $('#player .position-info')[0];
        $('#player .play').click(() => this.play());
        $('#player .pause').click(() => this.pause());
        $('#player .stop').click(() => this.stop());
        $('#player .prev').click(() => this.prev());
        $('#player .next').click(() => this.next());
        this.seek_widget.min = 0;
        this.seek_widget.max = 0;
        this.current_frame = 0;
        this.frame_cnt = 0;
        $(this.seek_widget).bind('input', (event) => {
                this.current_frame = parseInt(event.target.value);
                this.loadCurrentFrame();
                console.log("seek change", this.current_frame);
            }
        );
        this.startTime = null;
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
                    this.frame_interval = 1000 / ((is_ntsc ? 262 * 60 : 312 * 50) / fastplay);
                    if(this.frame_interval < 4) {
                        console.warn("unsupported (too small) frame interval:", this.frame_interval, "ms");
                        this.data = new Uint8Array();
                    }
                } else if (is_ntsc) {
                    this.frame_interval = 1000 / 60;
                } else {
                    this.frame_interval = 1000 / 50;
                }
                console.log("frame interval:", this.frame_interval)
                this.frame_cnt = Math.floor(this.data.length / 9);
                this.seek_widget.max = this.frame_cnt > 0 ? this.frame_cnt - 1 : 0;
                this.current_frame = 0;
                this.updatePosition();
                let is_ok = this.data.length > 0;
                if(is_ok) this.play();
                return is_ok;
            }
            ptr++;
        }
        console.warn("cannot locate data section");
        return false;
    }
    playFrame() {
        this.current_frame = (this.current_frame + 1) % this.frame_cnt;
        this.loadCurrentFrame();
    }
    loadCurrentFrame() {
        let regs = this.data.slice(this.current_frame * 9, this.current_frame * 9 + 9);
        window.pokey_port.postMessage(regs);
        this.updatePosition();
        this._send_pokey_regs_event(regs);
    }
    _send_pokey_regs_event(regs) {
        let event = new Event("pokey_regs");
        event.regs = regs;
        window.dispatchEvent(event);
    }
    stop() {
        if(this.interval) {
            clearInterval(this.interval);
            this.interval = null;
        }
        this.startTime = null;
        this.current_frame = 0;
        this.updatePosition();
        let regs = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        window.pokey_port.postMessage(regs);
        this._send_pokey_regs_event(regs);
    }
    play() {
        if(this.startTime == null) {
            this.startTime = window.audio_context.currentTime;
            console.log(this.startTime);
        }
    }
    pause() {
        if(this.interval) {
            clearInterval(this.interval);
            this.interval = null;
            this.loadCurrentFrame();
        }
    }
    isPaused() {
        return !this.interval;
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
    updatePosition() {
        this.seek_widget.value = this.current_frame;
        this.position_info.innerText = `${this.current_frame} / ${this.frame_cnt}`
    }
}
