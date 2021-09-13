const CPU_CYCLES_PER_SEC = 312 * 114 * 50;
// const POKEY_FREQ = 312 * 114 * 50;
// const POKEY_FREQ = 44100 * 40;
// const OUT_FREQ = 44100;

const OUT_FREQ = 48000;
const M = 37;
const POKEY_FREQ = 48000 * M;

const REC_BUF_SIZE = 9 * 50;

class POKEY {
  constructor(index) {
    this.index = index
    this.filter = new FIRFilter(FIR_37_to_1);
    this.clock_cnt = 0;
    
    this.set_audctl(0);

    this.audf = [0, 0, 0, 0];
    this.audc = [0, 0, 0, 0];
    this.cnt = [0, 0, 0, 0];
    this.square_output = [0, 0, 0, 0];
    this.output = [0, 0, 0, 0];

    function poly_array(gen) {
      let array = new Int8Array(gen.size())
      for(var i=0; i<gen.size(); i++) {
        array[i] = gen.next();
      }
      return array;
    }
    this.poly_4 = poly_array(new Poly4())
    // console.log("poly_4:", this.poly_4)
    this.poly_5 = [1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0]
    // this.poly_5 = poly_array(new Poly5())
    console.log("poly_5:", this.poly_5)
    this.poly_9 = poly_array(new Poly9())
    this.poly_17 = poly_array(new Poly17())
    this.cycle_cnt = 0;
  }
  set_audctl(value) {
    this.audctl = value;
    this.fast_1 = (value & 0x40) > 0;
    this.fast_3 = (value & 0x20) > 0;
    this.link12 = (value & 0x10) > 0;
    this.link34 = (value & 0x8) > 0;
    this.clock_period = value & 1 ? 114 : 28;
    this.hipass1 = (value & 4) > 0;
    this.hipass2 = (value & 2) > 0;
    this.hipass1_flipflop = 1;
    this.hipass2_flipflop = 1;
  }
  
  set_audf(index, value) {
    this.audf[index] = value;
  }

  set_audc(index, value) {
    this.audc[index] = value;
  }

  get_poly_output(k, poly) {
    return poly[(this.cycle_cnt + k) % poly.length];
  }

  get_output(k) {
    let audc = this.audc[k];
    if(audc & 0x20) {
      return this.square_output[k];
    } else {
      if(audc & 0x40) {
        return this.get_poly_output(k, this.poly_4)
      } else {
        if(this.audctl & 0x80) {
          return this.get_poly_output(k, this.poly_9)
        } else {
          return this.get_poly_output(k, this.poly_17)
        }
      }
    }
  }

  set_output(k) {
    this.square_output[k] = (~this.square_output[k]) & 1
    if((this.audc[k] & 0x80) || this.get_poly_output(k, this.poly_5)) {
      this.output[k] = this.get_output(k)
    }
  }

  reload_single(k) {
    let fast_delay = (k == 0 && this.fast_1 || k == 2 && this.fast_3 ? 3 : 0)
    this.cnt[k] = this.audf[k] + fast_delay
    this.set_output(k)
  }

  reload_linked(k) {
    let cnt = this.audf[k] + 256 * this.audf[k + 1] + 6;
    this.cnt[k] = cnt & 0xff
    this.cnt[k + 1] = cnt >> 8;
    this.set_output(k + 1)
  }

  tick() {
    if(!this.hipass1) this.hipass1_flipflop = 1;
    if(!this.hipass2) this.hipass2_flipflop = 1;

    for (let j=0; j < M; j++) {
      this.clock_cnt -= 1;
      let clock_underflow = this.clock_cnt < 0;
      if(clock_underflow) {
        this.clock_cnt = this.clock_period - 1;
      }

      if(!this.link12) {
        if(this.fast_1 || clock_underflow) {
          this.cnt[0] -= 1;
          if(this.cnt[0] < 0) this.reload_single(0)
        }
        if(clock_underflow) {
          this.cnt[1] -= 1;
          if(this.cnt[1] < 0) this.reload_single(1)
        }
      } else {
        if(this.fast_1 || clock_underflow) {
          this.cnt[0] -= 1;
          if(this.cnt[0] < 0) {
            this.cnt[0] = 255;
            this.set_output(0);
            this.cnt[1] -= 1;
            if(this.cnt[1] < 0) this.reload_linked(0);
          }
        }
      }
      if(!this.link34) {
        if(this.fast_3 || clock_underflow) {
          this.cnt[2] -= 1;
          if(this.cnt[2] < 0) {
            this.reload_single(2)
            if(this.hipass1) {
              // this.hipass1_flipflop = this.output[0]
              this.set_output(0);
            }
          }
        }
        if(clock_underflow) {
          this.cnt[3] -= 1;
          if(this.cnt[3] < 0) {
            this.reload_single(3)
            if(this.hipass2) {
              // this.hipass2_flipflop = this.output[1]
              this.set_output(1);
            }
          }
        }
      } else {
        if(this.fast_3 || clock_underflow) {
          this.cnt[2] -= 1;
          if(this.cnt[2] < 0) {
            // what about hipass1 / hipass2 here?
            this.cnt[2] = 255;
            this.set_output(2)
            this.cnt[3] -= 1;
            if(this.cnt[3] < 0) this.reload_linked(2);
          }
        }
      }

      this.cycle_cnt += 1;

      let vol_only = n => (this.audc[n] >> 4) & 1
      let vol = n => this.audc[n] & 15

      let ch1 = 1 & (this.hipass1_flipflop ^ this.output[0]) | vol_only(0)
      let ch2 = 1 & (this.hipass2_flipflop ^ this.output[1]) | vol_only(1)
      let ch3 = 1 & this.output[2] | vol_only(2)
      let ch4 = 1 & this.output[3] | vol_only(3)

      let normalize = vol => vol / 60.0
      let normalizeAltirra = vol => (1.0 - Math.exp(-2.9 * (vol / 60.0))) / (1.0 - Math.exp(-2.9))
      let sample = normalizeAltirra(ch1 * vol(0) + ch2 * vol(1) + ch3 * vol(2) + ch4 * vol(3))
      this.filter.add_sample(sample);
    }
    return this.filter.get();
  }
}


class POKEYProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.stereo_cnt = 0;
    this.is_stereo = false;
    
    this.pokey = [
      new POKEY('L'),
      new POKEY('R'),
    ]

    this.input_dt = null;
    this.lastSyncTime = null;
    this.is_playing = false;
    this.buffer = [];
    this.buffer_pos = 0;
    this.sampleCnt = 0;
    this.volume = 0.5;

    this.minLatency = 0.02;
    this.maxLatency = 0.1;

    this.port.onmessage = (e) => {
      if(e.data == "clear_buffer") {
        this.buffer = [];
      } else if(e.data.length >= 3) {
        if(this.is_playing) {
          let minTimeAhead = this.sampleCnt / sampleRate + this.minLatency;
          let maxTimeAhead = this.sampleCnt / sampleRate + this.maxLatency;
          if(this.input_dt == null) {
            this.input_dt = minTimeAhead - e.data[2];
          }
          let min_input_t = e.data[2] + this.input_dt;
          let max_input_t = e.data[e.data.length - 1] + this.input_dt;
          if(min_input_t < minTimeAhead) {
            this.input_dt += (minTimeAhead - min_input_t);
            // console.log("input too slow, syncing", (minTimeAhead - min_input_t))
          } else if(max_input_t > maxTimeAhead) {
            this.input_dt -= (max_input_t - maxTimeAhead);
            // console.log("input too fast, syncing", (max_input_t - maxTimeAhead))
          }
          for(var i=2; i<e.data.length; i+=3) {
            e.data[i] += this.input_dt;
          }
          this.buffer = this.buffer.concat(e.data);
        }
      }
    }
  }

  set_stereo(is_stereo) {
    if(is_stereo ^ this.is_stereo) {
      console.info("is_stereo: ", is_stereo);
    }
    this.is_stereo = is_stereo;
  }

  processEvents() {
    var pokey_lr = 0;

    while(
      this.buffer_pos < this.buffer.length
      && this.buffer[this.buffer_pos + 2] <= this.sampleCnt / sampleRate
    ) {
      var index = this.buffer[this.buffer_pos];
      let value = this.buffer[this.buffer_pos + 1];
      let pokey_idx = this.pokey.length == 1 ? 0 : (index >> 4) & 1;
      index = index & 0xf;
      pokey_lr |= (pokey_idx + 1)
      if(index == 8) {
        this.pokey[pokey_idx].set_audctl(value)
      } else if((index & 1) == 0) {
        this.pokey[pokey_idx].set_audf(index >> 1, value);
      } else {
        this.pokey[pokey_idx].set_audc(index >> 1, value);
      }
      this.buffer_pos += 3;
    }
    if(pokey_lr == 3) {
      this.stereo_cnt += 1;
      if(this.stereo_cnt > 20) {
        this.stereo_cnt = 20;
        this.set_stereo(true);
      }
    } else if(pokey_lr == 1) {
      this.stereo_cnt -= 1;
      if(this.stereo_cnt < 0) {
        this.stereo_cnt = 0;
        this.set_stereo(false);
      }
    }
  }

  process (inputs, outputs, parameters) {
    this.is_playing = true;

    const output = outputs[0]
    for(let i=0; i < output[0].length; i++) {
      this.processEvents();
      output[0][i] = this.pokey[0].tick() * this.volume;
      if(this.is_stereo) {
        output[1][i] = this.pokey.length == 2 ? this.pokey[1].tick() * this.volume : output[0][i]
      } else {
        output[1][i] = output[0][i]
      }

      this.sampleCnt += 1;
    }

    if(this.buffer_pos > 0) {
      this.buffer.splice(0, this.buffer_pos);
      this.buffer_pos = 0;
    }
    // if(this.buffer.length > 0 && (this.lastSyncTime == null || currentTime - this.lastSyncTime >= 1.0)) {
    //   this.lastSyncTime = currentTime;
    //   console.log("buf len: ", this.buffer.length, " buffered time: ", this.buffer[this.buffer.length-1] - this.buffer[2]);
    // }
    return true
  }
}

registerProcessor('POKEY', POKEYProcessor)

class PolyGenerator {
  constructor(n_bits) {
    this.n_bits = n_bits
    this.highest_bit = 1 << (n_bits - 1)
    this.value = this.highest_bit
  }
  next() {
    let v = this.value & 1
    this.value = this.compute(this.value, this.n_bits, this.highest_bit)
    return v
  }
  size() {
    return 2 ** this.n_bits - 1
  }
}

class Poly4 extends PolyGenerator {
  constructor() {
    super(4)
  }
  compute(v, n_bits, highest_bit) {
    return (v >> 1) + (((v << (n_bits-1)) ^ (v << (3-1))) & highest_bit)
  }
}

class Poly5 extends PolyGenerator {
  constructor() {
    super(5)
  }
  compute(v, n_bits, highest_bit) {
    return (v >> 1) + (((v << (n_bits-1)) ^ (v << (3-1))) & highest_bit)
  }
}

class Poly9 extends PolyGenerator {
  constructor() {
    super(9)
  }
  compute(v, n_bits, highest_bit) {
    return (v >> 1) + (((v << (n_bits-1)) ^ (v << (4-1))) & highest_bit)
  }
}

class Poly17 extends PolyGenerator {
  constructor() {
    super(17)
  }
  compute(v, n_bits, highest_bit) {
    return (v >> 1) + (((v << (n_bits-1)) ^ (v << (12-1))) & highest_bit)
  }
}

class FIRFilter {
  constructor(coefficients) {
    this.coefficients = coefficients;
    this.buffer = new Float32Array(coefficients.length);
    this.current_pos = 0;
  }
  add_sample(value) {
    this.buffer[this.current_pos] = value;
    this.current_pos = (this.current_pos + 1) % this.buffer.length;
  }
  get_last() {
    return this.buffer[(this.current_pos + this.buffer.length - 1) % this.buffer.length]
  }
  get() {
    let len = this.buffer.length;
    let acc = 0.0;
    var j = this.current_pos;
    for(var i = 0; i < len; i++) {
      acc += this.coefficients[i] * this.buffer[j];
      j += 1;
      if(j >= len) {
        j = 0;
      }
    }
    return acc;
  }
}

const FIR_37_to_1 = [
  -0.09349821507735248,
  0.003666910567355903,
  0.0036151277883324537,
  0.0035799221411175353,
  0.003552242647927605,
  0.003542827033881285,
  0.0035426515413241907,
  0.0035520735940582087,
  0.003569618013347629,
  0.0035974373631003486,
  0.003632708493515057,
  0.0036738655873837325,
  0.0037189575861326573,
  0.0037681053189869747,
  0.003819940339882748,
  0.0038732029974407676,
  0.003926241364999738,
  0.0039780781033141015,
  0.004027204953874909,
  0.004072093429640854,
  0.004111633482933724,
  0.004145008295488616,
  0.004171330704066523,
  0.004189024633557977,
  0.004197313249896553,
  0.004195049333993395,
  0.004181436710680956,
  0.004154188589397751,
  0.004113548632279398,
  0.004059419988684275,
  0.003993299227443386,
  0.003910807222983513,
  0.003811898654149772,
  0.0036944665141468605,
  0.003570389880502825,
  0.0034251083331524102,
  0.0032891969007221066,
  0.0029434362816407223,
  0.0029359689430925088,
  0.0027372083348269627,
  0.0024994585198891057,
  0.0022389914495120605,
  0.001972732848411926,
  0.001697343014595121,
  0.001414783902719725,
  0.0011247825923963559,
  0.0008304109705764811,
  0.0005304666712575737,
  0.00022504290225006584,
  -0.00008589242788670133,
  -0.00040018439806973987,
  -0.0007168318959069872,
  -0.0010345781126983354,
  -0.001352288789958951,
  -0.0016679911664585927,
  -0.0019799853146573536,
  -0.0022865329986336193,
  -0.002585370288116768,
  -0.0028740990267505048,
  -0.0031504956628552066,
  -0.0034133098994903795,
  -0.0036603785887257922,
  -0.0038899840990163485,
  -0.0040999182032089975,
  -0.004289874078047064,
  -0.004456711316352827,
  -0.004597927496009218,
  -0.004709172105291624,
  -0.004793945555080395,
  -0.004850934034146558,
  -0.004882226702754321,
  -0.0048676915880072336,
  -0.004830724267858526,
  -0.004744211670717836,
  -0.004679845752613664,
  -0.0044872209617146264,
  -0.00430234915359917,
  -0.00409038228776031,
  -0.003844927750717906,
  -0.0035544479131191156,
  -0.0032232880186703053,
  -0.002852046995559706,
  -0.0024431940709126166,
  -0.0019959453275516293,
  -0.001513042408400861,
  -0.0009960858912993393,
  -0.0004468586472420972,
  0.00013468497575733034,
  0.000747343502326168,
  0.0013898357056942158,
  0.002060545939187186,
  0.0027583339521850565,
  0.0034815655317932417,
  0.0042282532497063706,
  0.004996676470669485,
  0.005784833181478376,
  0.006590326243003574,
  0.007409498803939983,
  0.008239792436886467,
  0.009078269029911153,
  0.009922770498384883,
  0.01076893124527478,
  0.011615595553595912,
  0.012460805625966303,
  0.013305023583902025,
  0.014138892161398034,
  0.014958547931464929,
  0.015754734373077763,
  0.016556371762352782,
  0.017321310785546867,
  0.01807662270944155,
  0.018778566736118836,
  0.019493785943272113,
  0.020160473805793944,
  0.020789547478532025,
  0.021381320969019128,
  0.021942018077760837,
  0.02246557368980795,
  0.022949096520486198,
  0.023388683621208155,
  0.023784029052037404,
  0.02413238032215285,
  0.024432325076989975,
  0.02468253796689992,
  0.02488349665568337,
  0.02503473364515759,
  0.02513582054039397,
  0.025186284074643815,
  0.025186284074643815,
  0.02513582054039397,
  0.02503473364515759,
  0.02488349665568337,
  0.02468253796689992,
  0.024432325076989975,
  0.02413238032215285,
  0.023784029052037404,
  0.023388683621208155,
  0.022949096520486198,
  0.02246557368980795,
  0.021942018077760837,
  0.021381320969019128,
  0.020789547478532025,
  0.020160473805793944,
  0.019493785943272113,
  0.018778566736118836,
  0.01807662270944155,
  0.017321310785546867,
  0.016556371762352782,
  0.015754734373077763,
  0.014958547931464929,
  0.014138892161398034,
  0.013305023583902025,
  0.012460805625966303,
  0.011615595553595912,
  0.01076893124527478,
  0.009922770498384883,
  0.009078269029911153,
  0.008239792436886467,
  0.007409498803939983,
  0.006590326243003574,
  0.005784833181478376,
  0.004996676470669485,
  0.0042282532497063706,
  0.0034815655317932417,
  0.0027583339521850565,
  0.002060545939187186,
  0.0013898357056942158,
  0.000747343502326168,
  0.00013468497575733034,
  -0.0004468586472420972,
  -0.0009960858912993393,
  -0.001513042408400861,
  -0.0019959453275516293,
  -0.0024431940709126166,
  -0.002852046995559706,
  -0.0032232880186703053,
  -0.0035544479131191156,
  -0.003844927750717906,
  -0.00409038228776031,
  -0.00430234915359917,
  -0.0044872209617146264,
  -0.004679845752613664,
  -0.004744211670717836,
  -0.004830724267858526,
  -0.0048676915880072336,
  -0.004882226702754321,
  -0.004850934034146558,
  -0.004793945555080395,
  -0.004709172105291624,
  -0.004597927496009218,
  -0.004456711316352827,
  -0.004289874078047064,
  -0.0040999182032089975,
  -0.0038899840990163485,
  -0.0036603785887257922,
  -0.0034133098994903795,
  -0.0031504956628552066,
  -0.0028740990267505048,
  -0.002585370288116768,
  -0.0022865329986336193,
  -0.0019799853146573536,
  -0.0016679911664585927,
  -0.001352288789958951,
  -0.0010345781126983354,
  -0.0007168318959069872,
  -0.00040018439806973987,
  -0.00008589242788670133,
  0.00022504290225006584,
  0.0005304666712575737,
  0.0008304109705764811,
  0.0011247825923963559,
  0.001414783902719725,
  0.001697343014595121,
  0.001972732848411926,
  0.0022389914495120605,
  0.0024994585198891057,
  0.0027372083348269627,
  0.0029359689430925088,
  0.0029434362816407223,
  0.0032891969007221066,
  0.0034251083331524102,
  0.003570389880502825,
  0.0036944665141468605,
  0.003811898654149772,
  0.003910807222983513,
  0.003993299227443386,
  0.004059419988684275,
  0.004113548632279398,
  0.004154188589397751,
  0.004181436710680956,
  0.004195049333993395,
  0.004197313249896553,
  0.004189024633557977,
  0.004171330704066523,
  0.004145008295488616,
  0.004111633482933724,
  0.004072093429640854,
  0.004027204953874909,
  0.0039780781033141015,
  0.003926241364999738,
  0.0038732029974407676,
  0.003819940339882748,
  0.0037681053189869747,
  0.0037189575861326573,
  0.0036738655873837325,
  0.003632708493515057,
  0.0035974373631003486,
  0.003569618013347629,
  0.0035520735940582087,
  0.0035426515413241907,
  0.003542827033881285,
  0.003552242647927605,
  0.0035799221411175353,
  0.0036151277883324537,
  0.003666910567355903,
  -0.09349821507735248
]