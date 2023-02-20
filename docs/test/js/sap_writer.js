function concat_arrays(arrays) {
    let totalLength = arrays.reduce((acc, value) => acc + value.length, 0);
    let result = new Uint8Array(totalLength);
    let length = 0;
    for(let array of arrays) {
          result.set(array, length);
          length += array.length;
    }
    return result;
}

function time_str(t) {
    let minutes = Math.floor(t / 60);
    let seconds = (t - minutes * 60).toFixed(2)
    return `${minutes}:${seconds < 10 ? '0' + seconds : seconds}`
}


export class SAPWriter {
    constructor(is_stereo, trim) {
        this.trim = trim
        this.frames_per_sec = 50;  // TODO
        this.frame_cnt = 0;
        this.frame_size = is_stereo ? 18 : 9
        this.is_stereo = is_stereo
        this.pokey_regs = new Uint8Array(is_stereo ? 32 : 16)
        this.pokey1_sap_regs = this.pokey_regs.subarray(0, 9)
        if(is_stereo)
            this.pokey2_sap_regs = this.pokey_regs.subarray(16, 16 + 9);

        this.out_buffer = new Uint8Array(65536)
        this.out_buffer_used = 0
    }
    is_zero_volume() {
        return (
            !(this.pokey1_sap_regs[1] & 0xf)
            && !(this.pokey1_sap_regs[3] & 0xf)
            && !(this.pokey1_sap_regs[5] & 0xf)
            && !(this.pokey1_sap_regs[7] & 0xf)
            && (
                !this.is_stereo || (
                    !(this.pokey2_sap_regs[1] & 0xf)
                    && !(this.pokey2_sap_regs[3] & 0xf)
                    && !(this.pokey2_sap_regs[5] & 0xf)
                    && !(this.pokey2_sap_regs[7] & 0xf)
                )
            )
        )

    }
    handle_pokey_msg(msg) {
        for(var i=0; i<msg.length; i+=3) {
            let reg = msg[i + 0]
            let value = msg[i + 1]
            let ts = msg[i + 2]
            this.pokey_regs[reg % this.pokey_regs.length] = value
        }
        if(!this.trim || !this.is_zero_volume()) {
            this.trim = false
            this.append(this.pokey1_sap_regs)
            if(this.is_stereo) {
                this.append(this.pokey2_sap_regs)
            }
            this.frame_cnt += 1
            this.send_event()
        }
    }

    append(array) {
        if(this.out_buffer.length - this.out_buffer_used < array.length) {
            this.resize_out_buffer()
        }
        this.out_buffer.set(array, this.out_buffer_used)
        this.out_buffer_used += array.length
    }

    resize_out_buffer() {
        let new_buffer = new Uint8Array(this.out_buffer.length * 2)
        new_buffer.set(this.out_buffer)
        this.out_buffer = new_buffer
    }

    send_event(regs) {
        let event = new Event("sap_writer");
        event.data = {
            frame_cnt: this.frame_cnt,
            data_size: this.frame_size * this.frame_cnt,
            duration: time_str(this.frame_cnt / this.frames_per_sec),
        }
        window.dispatchEvent(event);
    }

    get_sap(additional_headers) {
        let duration_s = this.frame_cnt / this.frames_per_sec
        let headers = [
            "SAP",
            "TYPE R",
            ...(additional_headers || []),
            `TIME ${time_str(duration_s)}`,
            `FASTPLAY ${this.frames_per_sec == 50 ? 312 : 262}`,
            ...(this.is_stereo ? ["STEREO"] : []),
        ]
        let header = headers.join("\r\n") + "\r\n\r\n";
        let enc = new TextEncoder();
        let header_data = enc.encode(header);
        return concat_arrays([header_data, this.out_buffer.subarray(0, this.out_buffer_used)])
    }
}
