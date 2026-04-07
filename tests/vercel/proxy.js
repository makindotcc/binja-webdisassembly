const mockttp = require("mockttp");
const fs = require("fs");
const path = require("path");

const INTERCEPT_URL =
  "https://v0-hello-world-site-zeta.vercel.app/.well-known/vercel/security/static/challenge.v2.min.js";

const USE_ORIGINAL_WASM = false; // toggle: true = original .wasm, false = decompiled JS

function buildBundle() {
  if (USE_ORIGINAL_WASM) {
    // Use the original .wasm with debug logging only
    return fs.readFileSync(
      path.join(__dirname, "challenge_original.js"),
      "utf-8"
    );
  }

  const wasmJs = fs.readFileSync(
    path.join(__dirname, "challenge_wasm.js"),
    "utf-8"
  );
  const challengeJs = fs.readFileSync(
    path.join(__dirname, "challenge_decompiled.js"),
    "utf-8"
  );

  // Wrap challenge_wasm.js in an IIFE that returns the exports object
  // (replacing CommonJS module.exports with a return)
  const wrappedWasm = `var __wasmModule = (function() {\n  var module = { exports: {} };\n${wasmJs}\n  return module.exports;\n})();\n`;

  // Replace require("./challenge_wasm.js") with __wasmModule
  const patchedChallenge = challengeJs.replace(
    /require\(["']\.\/challenge_wasm\.js["']\)/,
    "__wasmModule"
  );

  return wrappedWasm + patchedChallenge;
}

async function main() {
  const https = await mockttp.generateCACertificate();
  const server = mockttp.getLocal({ https });

  await server.start(8080);

  await server
    .forGet(INTERCEPT_URL)
    .thenCallback(() => {
      console.log(`Intercepted request to ${INTERCEPT_URL}`);
      return {
        statusCode: 200,
        headers: { "Content-Type": "application/javascript" },
        body: buildBundle(),
      };
    })
  // .thenReply(200, bundle, {
  //   "Content-Type": "application/javascript",
  // });

  await server.forUnmatchedRequest().thenPassThrough();

  console.log(`Proxy running on ${server.url}`);

  process.on("SIGINT", async () => {
    await server.stop();
    process.exit(0);
  });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
