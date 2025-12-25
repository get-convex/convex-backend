// The code in this file is adapted from
// https://github.com/aws-samples/sample-lambda-network-monitor
//
// The original code has MIT License - the text of which is replicated here.
//
// MIT No Attribution
//
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

import fs from "fs";

type InvokeData = {
  txBytes: number;
};

let prevInvokeData: InvokeData = {
  txBytes: 0,
};

// Subject to change, make sure you scan all devices
const DEVICE_NAME = "vint_runtime";
export function countEgressBytes() {
  const newData = getNetworkDeviceData(DEVICE_NAME);
  if (!newData) {
    return 0;
  }
  //const lastRxBytes = prevInvokeData.rxBytes;
  const lastTxBytes = prevInvokeData.txBytes;

  //const rxBytesDiff = newData.rxBytes - lastRxBytes;
  const txBytesDiff = newData.txBytes - lastTxBytes;
  prevInvokeData = newData;
  return txBytesDiff;
}

function getNetworkDeviceData(device: string): InvokeData | undefined {
  let file;
  try {
    file = fs.readFileSync("/proc/net/dev", "utf8");
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") {
      return undefined;
    }
    throw err;
  }
  const lines = file.split("\n");
  const line = lines.filter((line) => line.includes(device))[0];
  if (!line) {
    return undefined;
  }
  const parts = line.trim().split(/\s+/g);

  //const name = parts[0].substring(0, parts[0].length - 1);
  //const rxBytes = parseInt(parts[1]);
  // const rxPackets = parseInt(parts[2]);
  // const rxErrors = parseInt(parts[3]);
  // const rxDrop = parseInt(parts[4]);
  // const rxFifo = parseInt(parts[5]);
  // const rxFrame = parseInt(parts[6]);
  // const rxCompressed = parseInt(parts[7]);
  // const rxMulticast = parseInt(parts[8]);
  const txBytes = parseInt(parts[9]);
  // const txPackets = parseInt(parts[10]);
  // const txErrors = parseInt(parts[11]);
  // const txDrop = parseInt(parts[12]);
  // const txFifo = parseInt(parts[13]);
  // const txColls = parseInt(parts[14]);
  // const txCarrier = parseInt(parts[15]);
  // const txCompressed = parseInt(parts[16]);
  return {
    txBytes,
  };
}
