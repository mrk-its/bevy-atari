// clock frequency for PAL is 312 * 114 * 50 = 1778400
// 48000 * 37 = 1776000  (relative error: 0.00135)
// 44100 * 40 = 1764000  (relative error: 0.0081)
// so both values are probably acceptable

// Altirra Hardware Reference Manual,
// D.3 First amplifier stage, page 388
const HIGH_PASS_TIME_CONST = 0.0026

class POKEY {
  constructor(index) {
    this.buffer = [];
    this.buffer_pos = 0;

    this.index = index
    if (sampleRate == 48000) {
      this.fir_filter = new FIRFilter(FIR_37_to_1);
      this.divider = 37
    } else if (sampleRate == 44100) {
      this.fir_filter = new Filter_Cascade_40_1();
      this.divider = 40
    } else if (sampleRate == 56000) {
      this.fir_filter = new Filter_Cascade_32_1();
      this.divider = 32
    } else {
      let err = `invalid sample rate ${sampleRate}`
      console.error(err)
      throw err
    }

    this.high_pass_filter = new HighPassFilter(HIGH_PASS_TIME_CONST, sampleRate);
    this.clock_cnt = 0;

    this.set_audctl(0);

    this.audf = [0, 0, 0, 0];
    this.audc = [0, 0, 0, 0];
    this.cnt = [0, 0, 0, 0];
    this.square_output = [0, 0, 0, 0];
    this.output = [0, 0, 0, 0];
    this.console = 0;

    function poly_array(gen) {
      let array = new Int8Array(gen.size())
      for (var i = 0; i < gen.size(); i++) {
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

  feed(data) {
    this.buffer = this.buffer.concat(data);
  }

  truncateBuffer() {
    if (this.buffer_pos > 0) {
      this.buffer.splice(0, this.buffer_pos);
      this.buffer_pos = 0;
    }
  }

  processEvents(currentFrame) {
    while (
      this.buffer_pos < this.buffer.length
      && this.buffer[this.buffer_pos + 2] <= currentFrame / sampleRate
    ) {
      var index = this.buffer[this.buffer_pos] & 0xf;
      let value = this.buffer[this.buffer_pos + 1];
      if (index == 8) {
        this.set_audctl(value)
      } else if (index == 9) {
        this.set_console(value);
      } else if ((index & 1) == 0) {
        this.set_audf(index >> 1, value);
      } else {
        this.set_audc(index >> 1, value);
      }
      this.buffer_pos += 3;
    }
  }

  set_audctl(value) {
    this.audctl = value;
    this.fast_1 = (value & 0x40) > 0;
    this.fast_3 = (value & 0x20) > 0;
    this.link12 = (value & 0x10) > 0;
    this.link34 = (value & 0x8) > 0;
    this.clock_period = value & 1 ? 114 : 28;
    this.hipass1 = (value >> 2) & 1;
    this.hipass2 = (value >> 1) & 1;
    this.hipass1_flipflop |= !this.hipass1;
    this.hipass2_flipflop |= !this.hipass2;
  }

  set_audf(index, value) {
    this.audf[index] = value;
  }

  set_audc(index, value) {
    this.audc[index] = value;
  }
  set_console(value) {
    this.console = value & 1;
  }
  get_poly_output(k, poly) {
    return poly[(this.cycle_cnt + k) % poly.length];
  }

  get_output(k) {
    let audc = this.audc[k];
    if (audc & 0x20) {
      return this.square_output[k];
    } else {
      if (audc & 0x40) {
        return this.get_poly_output(k, this.poly_4)
      } else {
        if (this.audctl & 0x80) {
          return this.get_poly_output(k, this.poly_9)
        } else {
          return this.get_poly_output(k, this.poly_17)
        }
      }
    }
  }

  set_output(k) {
    if (this.audc[k] & 0x80 || this.get_poly_output(k, this.poly_5)) {
      this.square_output[k] = (~this.square_output[k]) & 1
    }
    this.output[k] = this.get_output(k)
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

  get() {
    for (let j = 0; j < this.divider; j++) {
      this.clock_cnt -= 1;
      let clock_underflow = this.clock_cnt < 0;
      if (clock_underflow) {
        this.clock_cnt = this.clock_period - 1;
      }

      if (!this.link12) {
        if (this.fast_1 || clock_underflow) {
          this.cnt[0] -= 1;
          if (this.cnt[0] < 0) this.reload_single(0)
        }
        if (clock_underflow) {
          this.cnt[1] -= 1;
          if (this.cnt[1] < 0) this.reload_single(1)
        }
      } else {
        if (this.fast_1 || clock_underflow) {
          this.cnt[0] -= 1;
          if (this.cnt[0] < 0) {
            this.cnt[0] = 255;
            this.set_output(0);
            this.cnt[1] -= 1;
            if (this.cnt[1] < 0) this.reload_linked(0);
          }
        }
      }
      if (!this.link34) {
        if (this.fast_3 || clock_underflow) {
          this.cnt[2] -= 1;
          if (this.cnt[2] < 0) {
            this.reload_single(2)
            if (this.hipass1) {
              this.hipass1_flipflop = this.output[0]
            }
          }
        }
        if (clock_underflow) {
          this.cnt[3] -= 1;
          if (this.cnt[3] < 0) {
            this.reload_single(3)
            if (this.hipass2) {
              this.hipass2_flipflop = this.output[1]
            }
          }
        }
      } else {
        if (this.fast_3 || clock_underflow) {
          this.cnt[2] -= 1;
          if (this.cnt[2] < 0) {
            // what about hipass1 / hipass2 here?
            this.cnt[2] = 255;
            this.set_output(2)
            this.cnt[3] -= 1;
            if (this.cnt[3] < 0) this.reload_linked(2);
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
      let normalizeAltirra = vol => (1.0 - Math.exp(-2.9 * (vol / 64.0))) / (1.0 - Math.exp(-2.9))
      let sample = normalizeAltirra(ch1 * vol(0) + ch2 * vol(1) + ch3 * vol(2) + ch4 * vol(3) + this.console * 4)
      this.fir_filter.add_sample(sample);
    }
    return this.high_pass_filter.get(this.fir_filter.get());
  }
}


class POKEYProcessor extends AudioWorkletProcessor {

  static get parameterDescriptors() {
    return [{
      name: "gain",
      defaultValue: 1,
      minValue: 0,
      maxValue: 1,
      automationRate: "k-rate",
    }]
  }

  constructor(options) {
    super();
    this.has_stereo_output = options.outputChannelCount && options.outputChannelCount[0] > 1 || false
    this.is_stereo_input = true

    this.stereo_input_cnt = 0;

    this.pokey = [new POKEY('L')];

    this.port.onmessage = (e) => {
      this.setStereo(e.data.length > 1)
      if (e.data[0] && e.data[0].length) {
        this.pokey[0].feed(e.data[0])
      }
      if (e.data[1] && e.data[1].length) {
        this.pokey[1].feed(e.data[1])
      }
    }
  }

  setStereo(enable) {
    if (!this.has_stereo_output) return;
    let isEnabled = this.pokey.length > 1;
    if (enable ^ isEnabled) {
      console.log("setStereo:", enable)
      if (enable) {
        this.pokey.push(new POKEY('R'));
      } else {
        this.pokey.splice(1, 1);
      }
    }
  }

  set_stereo_input(is_stereo_input) {
    if (is_stereo_input ^ this.is_stereo_input) {
      console.info("is_stereo_input:", is_stereo_input);
      this.is_stereo_input = is_stereo_input;
    }
  }

  processEvents(currentFrame) {
    this.pokey.forEach(p => p.processEvents(currentFrame))
  }

  process(inputs, outputs, parameters) {
    let gain = parameters.gain[0];
    const output = outputs[0]
    for (let i = 0; i < output[0].length; i++) {
      this.processEvents(currentFrame + i);
      output[0][i] = this.pokey[0].get() * gain;
      if (output.length > 1) {
        if (this.is_stereo_input) {
          output[1][i] = this.pokey.length == 2 ? this.pokey[1].get() * gain : output[0][i]
        } else {
          output[1][i] = output[0][i]
        }
      }
    }
    this.pokey.forEach(p => p.truncateBuffer())
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
    return ((v + v)) + (~((v >> 2) ^ (v >> 3)) & 1)
  }
}

class Poly5 extends PolyGenerator {
  constructor() {
    super(5)
  }
  compute(v, n_bits, highest_bit) {
    return ((v + v)) + (~((v >> 2) ^ (v >> 4)) & 1)
  }
}

class Poly9 extends PolyGenerator {
  constructor() {
    super(9)
  }
  compute(v, n_bits, highest_bit) {
    return ((v >> 1)) + (((v << 8) ^ (v << 3)) & 0x100)
  }
}

class Poly17 extends PolyGenerator {
  constructor() {
    super(17)
  }
  compute(v, n_bits, highest_bit) {
    return ((v >> 1)) + (((v << 16) ^ (v << 11)) & 0x10000)
  }
}

class HighPassFilter {
  constructor(time_const, freq) {
    this.alpha = Math.exp((-1 / (time_const * freq)))
    this.prev_input = 0.0
    this.prev_output = 0.0
  }
  get(input) {
    this.prev_output = this.alpha * (this.prev_output + input - this.prev_input)
    this.prev_input = input
    return this.prev_output
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
    for (var i = 0; i < len; i++) {
      acc += this.coefficients[i] * this.buffer[j];
      j += 1;
      if (j >= len) {
        j = 0;
      }
    }
    return acc;
  }
}

class FIRHalfBandFilter extends FIRFilter {
  // [-0.03171533865036624, 0.0, 0.28171337137114716, 0.5, 0.28171337137114716, 0.0, -0.03171533865036624]
  get() {
    let len = this.buffer.length;
    let mid = len / 2 | 0;
    let acc = 0.5 * this.buffer[(this.current_pos + mid) % len];
    var j = this.current_pos;
    var k = (this.current_pos + this.coefficients.length - 1) % this.buffer.length
    for (var i = 0; i < mid; i += 2) {
      acc += this.coefficients[i] * (this.buffer[j] + this.buffer[k])
      j += 2;
      if (j >= len) j -= len;
      k -= 2;
      if (k < 0) k += len;
    }
    return acc;
  }
}

class Filter_Cascade_40_1 {
  constructor() {
    this.sample_cnt = 0

    this.fir2_1 = new FIRHalfBandFilter(FIR_HALF_BAND);
    this.fir2_2 = new FIRHalfBandFilter(FIR_HALF_BAND);
    this.fir2_3 = new FIRHalfBandFilter(FIR_HALF_BAND);
    this.fir5 = new FIRFilter(FIR_5_TO_1);
  }

  add_sample(value) {
    let i = ++this.sample_cnt
    this.fir2_1.add_sample(value)
    if (i % 2 == 0) {
      this.fir2_2.add_sample(this.fir2_1.get())
      if (i % 4 == 0) {
        this.fir2_3.add_sample(this.fir2_2.get())
        if (i % 8 == 0) {
          this.fir5.add_sample(this.fir2_3.get())
        }
      }
    }
  }

  get() {
    return this.fir5.get()
  }

}

class Filter_Cascade_32_1 {
  constructor() {
    this.sample_cnt = 0

    // Fs = 224000.0 * 8
    // Fpb = 40000.0
    // N=6

    this.fir2_1 = new FIRHalfBandFilter([-0.03171533865036624, 0.0, 0.28171337137114716, 0.5, 0.28171337137114716, 0.0, -0.03171533865036624]);

    // Fs = 224000.0 * 4
    // Fpb = 20000.0
    // N = 6
    this.fir2_2 = new FIRHalfBandFilter([-0.03171533865036624, 0.0, 0.28171337137114716, 0.5, 0.28171337137114716, 0.0, -0.03171533865036624]);

    // Fs = 224000.0 * 2
    // Fpb = 20000.0
    // N = 10
    this.fir2_3 = new FIRHalfBandFilter([0.006463652883377436, 0.0, -0.05057086353201489, 0.0, 0.2941083869114641, 0.5, 0.2941083869114641, 0.0, -0.05057086353201489, 0.0, 0.006463652883377436]);

    // Fs = 224000.0
    // Fpb = 20000.0
    // N = 10
    this.fir2_4 = new FIRHalfBandFilter([0.008617456033360315, 0.0, -0.055858590442562465, 0.0, 0.2973198443911924, 0.5, 0.2973198443911924, 0.0, -0.055858590442562465, 0.0, 0.008617456033360315]);

    // Fs = 112000.0
    // Fpb = 20000.0
    // Fsb = 28000.0
    // Apb = math.pow(10, -3)
    // Asb = 120
    // N = 39
    this.fir2_5 = new FIRFilter([-0.0008703171065093733, -0.001111550243261059, 0.0016791763634718098, 0.0065629513301804256, 0.006930696420325587, -0.0012804774882072276, -0.009570033303475958, -0.004193346911575355, 0.011788903142794962, 0.014881338450761158, -0.0069419479777090305, -0.026869669528549692, -0.008274709225594066, 0.0347121911105144, 0.03696484438079057, -0.029171360239427818, -0.08456317480317924, -0.011142070661437575, 0.1959897407866437, 0.3830518681272077, 0.3830518681272077, 0.1959897407866437, -0.011142070661437575, -0.08456317480317924, -0.029171360239427818, 0.03696484438079057, 0.0347121911105144, -0.008274709225594066, -0.026869669528549692, -0.0069419479777090305, 0.014881338450761158, 0.011788903142794962, -0.004193346911575355, -0.009570033303475958, -0.0012804774882072276, 0.006930696420325587, 0.0065629513301804256, 0.0016791763634718098, -0.001111550243261059, -0.0008703171065093733]);
    // this.fir2_5 = new FIRFilter([0.0002986011184819716, 0.0017501230334916774, 0.003576364326985988, 0.0061836441036239465, 0.008664561980965718, 0.010010372250045842, 0.00898313516300459, 0.004674075487226803, -0.002996357201063239, -0.012851927441860212, -0.02236390127251487, -0.027999848589650433, -0.026021136570137098, -0.013560916390046345, 0.010314480273422818, 0.04392225792080535, 0.0829279688371058, 0.12106550565283346, 0.1514812378307162, 0.16835694242530336, 0.16835694242530336, 0.1514812378307162, 0.12106550565283346, 0.0829279688371058, 0.04392225792080535, 0.010314480273422818, -0.013560916390046345, -0.026021136570137098, -0.027999848589650433, -0.02236390127251487, -0.012851927441860212, -0.002996357201063239, 0.004674075487226803, 0.00898313516300459, 0.010010372250045842, 0.008664561980965718, 0.0061836441036239465, 0.003576364326985988, 0.0017501230334916774, 0.0002986011184819716])
  }

  add_sample(value) {
    let i = ++this.sample_cnt
    this.fir2_1.add_sample(value)
    if (i % 2 == 0) {
      this.fir2_2.add_sample(this.fir2_1.get())
      if (i % 4 == 0) {
        this.fir2_3.add_sample(this.fir2_2.get())
        if (i % 8 == 0) {
          this.fir2_4.add_sample(this.fir2_3.get())
          if (i % 16 == 0) {
            this.fir2_5.add_sample(this.fir2_4.get())
          }
        }
      }
    }
  }

  get() {
    return this.fir2_5.get()
  }

}

const FIR_HALF_BAND = [0.006631578542146366, 0.0, -0.051031250383516566, 0.0, 0.29440207570898513, 0.5, 0.29440207570898513, 0.0, -0.051031250383516566, 0.0, 0.006631578542146366]
const FIR_5_TO_1 = [2.2055693741469678e-05, 0.00016140587951149585, 0.0004455947025289253, 0.0010027491986913189, 0.0019249046528200598, 0.003292700177846668, 0.005127062345683061, 0.007360044107222422, 0.009812053207538326, 0.012190930665028552, 0.014119154130111888, 0.015191082320631741, 0.015053869269425509, 0.013497498642671905, 0.010532963553996738, 0.006435844129913317, 0.0017367094269457202, -0.0028500511841174306, -0.006552965686958489, -0.008703832465625354, -0.008897095134987279, -0.007108265775657866, -0.003735936739822901, 0.0004535423291632218, 0.0044768183812528076, 0.007352674908280225, 0.008340990245320049, 0.007142213251758996, 0.004001258158233128, -0.0003205602839678702, -0.004710546325493503, -0.007975622821472588, -0.009156281544343947, -0.007802711848847073, -0.00413496541919464, 0.000966745057404005, 0.006154220399241479, 0.009949084817547832, 0.011146126195162328, 0.00917646467750611, 0.004325693794929819, -0.0022670044295789137, -0.008849484017916917, -0.013493348526984732, -0.014624220110211335, -0.011508122236123242, -0.004554623510358881, 0.004666013275846754, 0.013726246796241974, 0.019917492427708898, 0.020972903387322855, 0.015760095424924746, 0.004758484551244897, -0.009825300616758409, -0.024369438665459646, -0.03453989694364911, -0.036245472521959685, -0.02663411123433674, -0.004893820187556659, 0.027345440347776473, 0.06612941205977459, 0.10581952392674725, 0.14013585322134517, 0.16338524385008033, 0.1716074988924923, 0.16338524385008033, 0.14013585322134517, 0.10581952392674725, 0.06612941205977459, 0.027345440347776473, -0.004893820187556659, -0.02663411123433674, -0.036245472521959685, -0.03453989694364911, -0.024369438665459646, -0.009825300616758409, 0.004758484551244897, 0.015760095424924746, 0.020972903387322855, 0.019917492427708898, 0.013726246796241974, 0.004666013275846754, -0.004554623510358881, -0.011508122236123242, -0.014624220110211335, -0.013493348526984732, -0.008849484017916917, -0.0022670044295789137, 0.004325693794929819, 0.00917646467750611, 0.011146126195162328, 0.009949084817547832, 0.006154220399241479, 0.000966745057404005, -0.00413496541919464, -0.007802711848847073, -0.009156281544343947, -0.007975622821472588, -0.004710546325493503, -0.0003205602839678702, 0.004001258158233128, 0.007142213251758996, 0.008340990245320049, 0.007352674908280225, 0.0044768183812528076, 0.0004535423291632218, -0.003735936739822901, -0.007108265775657866, -0.008897095134987279, -0.008703832465625354, -0.006552965686958489, -0.0028500511841174306, 0.0017367094269457202, 0.006435844129913317, 0.010532963553996738, 0.013497498642671905, 0.015053869269425509, 0.015191082320631741, 0.014119154130111888, 0.012190930665028552, 0.009812053207538326, 0.007360044107222422, 0.005127062345683061, 0.003292700177846668, 0.0019249046528200598, 0.0010027491986913189, 0.0004455947025289253, 0.00016140587951149585, 2.2055693741469678e-05]
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
