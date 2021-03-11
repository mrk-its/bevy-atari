const CPU_CYCLES_PER_SEC = 312 * 114 * 50;
// const POKEY_FREQ = 312 * 114 * 50;
// const POKEY_FREQ = 44100 * 40;
// const OUT_FREQ = 44100;

const OUT_FREQ = 48000;
const M = 37;
const POKEY_FREQ = 48000 * M;

const REC_BUF_SIZE = 9 * 50;

class POKEY extends AudioWorkletProcessor {
  constructor() {
    super();
    this.recorded = null;
    this.port.onmessage = (e) => {
      if(e.data == "rec_start") {
        this.recorded = new Uint8Array(REC_BUF_SIZE);
        this.rec_ptr = 0;
        console.log("rec started");
      } else if(e.data == "rec_stop") {
        this.port.postMessage(this.recorded.slice(0, this.rec_ptr));
        this.recorded = null;
      } else {
        if(this.recorded != null) {
          for(var i=0; i<9; i++) {
            this.recorded[this.rec_ptr + i] = e.data[i];
          }
          this.rec_ptr = (this.rec_ptr + 9) % this.recorded.length;
          if(this.rec_ptr == 0) {
            this.port.postMessage(this.recorded);
          }
        }
        this.audctl = e.data[8]
        for(var i=0; i<4; i++) {
          this.audf[i] = e.data[i * 2];
          this.audc[i] = e.data[i * 2 + 1];
        }
      }
    }
    this.filter = new FIRFilter(FIR_37_to_1);
    this.out_t = 0;
    this.pokey_t = 0;
    this.clock_cnt = 0;
    this.clock_period = 28;

    this.total_cycles = 0;

    this.audctl = 0;

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
    this.poly_5 = poly_array(new Poly5())
    this.poly_9 = poly_array(new Poly9())
    this.poly_17 = poly_array(new Poly17())
    this.cycle_cnt = 0;
  }
  get_poly_output(k, poly) {
    return poly[(this.cycle_cnt + k) % poly.length];
  }
  get_output(k) {
    let audc = this.audc[k];
    let mask = audc & 0x80 ? 1 : this.get_poly_output(k, this.poly_5);
    if(audc & 0x20) {
      return mask & this.square_output[k];
    } else {
      if(audc & 0x40) {
        return mask & this.get_poly_output(k, this.poly_4)
      } else {
        if(this.audctl & 0x80) {
          return mask & this.get_poly_output(k, this.poly_9)
        } else {
          return mask & this.get_poly_output(k, this.poly_17)
        }
      }
    }
  }

  set_output(k) {
    this.output[k] = this.get_output(k);
  }

  reload_single(k) {
    this.cnt[k] = this.audf[k];
    this.square_output[k] = (~this.square_output[k]) & 1
  }

  reload_linked(k) {
    let cnt = this.audf[k] + 256 * this.audf[k + 1] + 6;
    this.cnt[k] = cnt & 0xff
    this.cnt[k + 1] = cnt >> 8;
    this.square_output[k + 1] = (~this.square_output[k + 1]) & 1
  }

  process (inputs, outputs, parameters) {
    const output = outputs[0]

    let fast_1 = (this.audctl & 0x40) > 0;
    let fast_3 = (this.audctl & 0x20) > 0;
    let link12 = (this.audctl & 0x10) > 0;
    let link34 = (this.audctl & 0x8) > 0;
    let clock_period = this.audctl & 1 ? 114 : 28;
    let hipass1 = (this.audctl & 4) > 0;
    let hipass2 = (this.audctl & 2) > 0;

    // if(hipass1) {
    //   console.log("hipass1 c0", this.audc[0], "f0", this.audf[0], "c2", this.audc[2], "f2", this.audf[2], this.audctl);
    // }
    // if(hipass2) {
    //   console.log("hipass2 c1", this.audc[1], "f1", this.audf[1], "c3", this.audc[3], "f3", this.audf[3], this.audctl);
    // }

    output.slice(0, 1).forEach(channel => {
      for (let i = 0; i < channel.length; i++) {
        for (let j=0; j < M; j++) {
          this.clock_cnt -= 1;
          let clock_underflow = this.clock_cnt < 0;
          if(clock_underflow) {
            this.clock_cnt = clock_period - 1;
          }

          if(!link12) {
            if(fast_1 || clock_underflow) {
              this.cnt[0] -= 1;
              if(this.cnt[0] < 0) {
                this.reload_single(0)
                this.output[0] = this.get_output(0)
              }
            }
            if(clock_underflow) {
              this.cnt[1] -= 1;
              if(this.cnt[1] < 0) {
                this.reload_single(1)
                this.output[1] = this.get_output(1)
              }
            }
          } else {
            if(fast_1 || clock_underflow) {
              this.cnt[0] -= 1;
              if(this.cnt[0] < 0) {
                this.cnt[0] = 255;
                this.square_output[0] = (~this.square_output[0]) & 1
                this.output[0] = this.get_output(0)
                this.cnt[1] -= 1;
                if(this.cnt[1] < 0) {
                  this.reload_linked(0);
                  this.output[1] = this.get_output(1);
                }
              }
            }
          }
          if(!link34) {
            if(fast_3 || clock_underflow) {
              this.cnt[2] -= 1;
              if(this.cnt[2] < 0) {
                this.reload_single(2)
                this.output[2] = this.get_output(2)
              }
            }
            if(clock_underflow) {
              this.cnt[3] -= 1;
              if(this.cnt[3] < 0) {
                this.reload_single(3)
                this.output[3] = this.get_output(3);
              }
            }
          } else {
            if(fast_3 || clock_underflow) {
              this.cnt[2] -= 1;
              if(this.cnt[2] < 0) {
                this.cnt[2] = 255;
                this.square_output[2] = (~this.square_output[2]) & 1
                this.output[2] = this.get_output(2)
                this.cnt[3] -= 1;
                if(this.cnt[3] < 0) {
                  this.reload_linked(2);
                  this.output[3] = this.get_output(3);
                }
              }
            }
          }

          let ch1_off = hipass1 && !this.square_output[2] ? 0 : 1;
          let ch2_off = hipass2 && !this.square_output[3] ? 0 : 1;

          this.cycle_cnt += 1;
          this.filter.add_sample(

            0.2 * (2 * (1 ^ this.output[0]) - 1) * ch1_off * (this.audc[0] & 15) / 15.0
            + 0.2 * (2 * (1 ^ this.output[1]) - 1) * ch2_off * (this.audc[1] & 15) / 15.0
            + 0.2 * (2 * this.output[2] - 1) * (link34 ? 0 : 1) * (this.audc[2] & 15) / 15.0
            + 0.2 * (2 * this.output[3] - 1) * (this.audc[3] & 15) / 15.0
          );
        }
        channel[i] = this.filter.get();
      }
    })
    this.total_cycles += 128;
    return true
  }
}

registerProcessor('POKEY', POKEY)

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