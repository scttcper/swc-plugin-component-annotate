import assert from "node:assert/strict";
import {mkdtemp, readFile, rm, writeFile} from "node:fs/promises";
import {tmpdir} from "node:os";
import path from "node:path";
import {test} from "node:test";

import {
  rspack,
  type Compiler,
  type Configuration,
  type Stats,
} from "@rspack/core";

const pluginPath = path.resolve(
  "target/wasm32-unknown-unknown/release/swc_plugin_component_annotate.wasm"
);

function runCompiler(compiler: Compiler): Promise<Stats> {
  return new Promise((resolve, reject) => {
    compiler.run((runError, stats) => {
      compiler.close(closeError => {
        const error = runError ?? closeError;
        if (error) {
          reject(error);
        } else if (stats) {
          resolve(stats);
        } else {
          reject(new Error("Rspack completed without returning compilation stats"));
        }
      });
    });
  });
}

test("runs through Rspack's built-in SWC loader", async t => {
  const fixtureDirectory = await mkdtemp(
    path.join(tmpdir(), "swc-plugin-component-annotate-rspack-")
  );
  t.after(() => rm(fixtureDirectory, {recursive: true, force: true}));

  const inputPath = path.join(fixtureDirectory, "Button.jsx");
  const outputPath = path.join(fixtureDirectory, "dist");
  await writeFile(
    inputPath,
    `
      function Button() {
        return <button />;
      }

      export {Button};
    `
  );

  const config: Configuration = {
    context: fixtureDirectory,
    devtool: false,
    entry: inputPath,
    mode: "development",
    module: {
      rules: [
        {
          test: /\.jsx$/,
          type: "javascript/auto",
          use: {
            loader: "builtin:swc-loader",
            options: {
              jsc: {
                parser: {syntax: "ecmascript", jsx: true},
                transform: {react: {runtime: "classic"}},
                experimental: {plugins: [[pluginPath, {}]]},
              },
            },
          },
        },
      ],
    },
    optimization: {minimize: false},
    output: {filename: "bundle.js", path: outputPath},
    target: "node",
  };

  const stats = await runCompiler(rspack(config));
  assert.equal(
    stats.hasErrors(),
    false,
    stats.toString({all: false, errors: true, errorDetails: true})
  );

  const bundle = await readFile(path.join(outputPath, "bundle.js"), "utf8");
  assert.match(bundle, /"data-component": "Button"/);
  assert.match(bundle, /"data-source-file": "Button\.jsx"/);
});
