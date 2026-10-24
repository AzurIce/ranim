/** One scene's baked mix and output clock. No renderer or UI ownership. */
export class AudioPlayback {
  constructor({ sampleRate, channels }, duration) {
    this._buffer = new AudioBuffer({ sampleRate, numberOfChannels: channels.length, length: channels[0].length });
    channels.forEach((channel, i) => this._buffer.copyToChannel(channel, i));
    this._duration = duration;
    this._context = null;
    this._source = null;
    this._gain = null;
    this._volume = 1;
    this._offset = 0;
    this._startedAt = 0;
  }

  play(time) {
    this.stop();
    this._offset = time;
    // Open/resume only in response to play, preserving browser user activation.
    this._context ??= new AudioContext({ latencyHint: "interactive" });
    const context = this._context;
    if (!this._gain) {
      this._gain = context.createGain();
      this._gain.gain.value = this._volume;
      this._gain.connect(context.destination);
    }
    this._startedAt = context.currentTime;
    const source = context.createBufferSource();
    source.buffer = this._buffer;
    source.connect(this._gain);
    source.onended = () => source.disconnect();
    try { source.start(this._startedAt, time, Math.max(0, Math.min(this._buffer.duration, this._duration) - time)); }
    catch (error) { source.disconnect(); throw error; }
    this._source = source;
    return context.resume().catch((error) => {
      // A rejected resume from a stopped/replaced voice must not pause its successor.
      if (this._source === source) { this.stop(); throw error; }
    });
  }

  position() {
    const context = this._context;
    if (!context) return this._offset;
    let clock = context.currentTime;
    // Align the visual with samples reaching the output, including device latency.
    const stamp = context.getOutputTimestamp?.();
    if (context.state === "running" && stamp?.performanceTime > 0) {
      clock = Math.min(clock, stamp.contextTime + Math.max(0, performance.now() - stamp.performanceTime) / 1000);
    }
    return Math.min(this._duration, this._offset + Math.max(0, clock - this._startedAt));
  }

  setVolume(volume) {
    this._volume = volume;
    this._gain?.gain.setTargetAtTime(volume, this._context.currentTime, 0.01);
  }

  stop() {
    if (!this._source) return;
    this._source.stop();
    this._source.disconnect();
    this._source = null;
  }

  dispose() {
    this.stop();
    const context = this._context;
    this._context = null;
    this._gain?.disconnect();
    this._gain = null;
    this._buffer = null;
    // resume() may still be pending when a viewport is removed.
    if (context && context.state !== "closed") void context.close().catch(() => {});
  }
}
