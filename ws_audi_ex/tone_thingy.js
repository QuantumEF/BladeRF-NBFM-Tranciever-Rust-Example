// import { waitForMessage } from "./tone_ws_thing.js";
// import { waitForMessage } from "./client.js";
function waitForMessage() {
  return new Promise((resolve) => {
    websocket.onmessage = (event) => {
      resolve(event.data);
    };
  });
}

class WsProcessor extends AudioWorkletProcessor {
  process(inputs, outputs, parameters) {
    const output = outputs[0];
    output.forEach(async (channel) => {
      const message = await waitForMessage();
      for (let i = 0; i < channel.length; i++) {
        channel[i] = Math.random() * 2 - 1;
      }
    });
    return true;
  }
}

registerProcessor("ws-processor", WsProcessor);