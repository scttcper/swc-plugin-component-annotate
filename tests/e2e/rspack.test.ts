import assert from "node:assert/strict";
import {mkdtemp, readFile, rm, writeFile} from "node:fs/promises";
import {tmpdir} from "node:os";
import path from "node:path";
import {test, type TestContext} from "node:test";

import {
  experiments,
  rspack,
  type Compiler,
  type Configuration,
  type Stats,
} from "@rspack/core";

import {pluginPath, sentryConfig, type PluginConfig} from "./helpers.ts";

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

/** Bundle a `Button` component through Rspack's built-in SWC loader. */
async function build(t: TestContext, pluginConfig: PluginConfig) {
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
                experimental: {plugins: [[pluginPath, pluginConfig]]},
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
  return {stats, bundlePath: path.join(outputPath, "bundle.js")};
}

test("runs through Rspack's built-in SWC loader", async t => {
  const {stats, bundlePath} = await build(t, {});
  assert.equal(
    stats.hasErrors(),
    false,
    stats.toString({all: false, errors: true, errorDetails: true})
  );

  const bundle = await readFile(bundlePath, "utf8");
  assert.match(bundle, /"data-component": "Button"/);
  assert.match(bundle, /"data-source-file": "Button\.jsx"/);
});

test("leaves out source attributes when Rspack's SWC has no filename", () => {
  // Rspack passes an empty filename rather than none
  const {code} = experiments.swc.transformSync(`function Button() { return <button />; }`, {
    jsc: {
      parser: {syntax: "ecmascript", jsx: true},
      experimental: {plugins: [[pluginPath, {}]]},
    },
  });

  assert.match(code, /"data-component": "Button"/);
  assert.doesNotMatch(code, /data-source-file/);
});

test("skips JSX the React Compiler hoists out of an svg in Rspack's SWC", () => {
  const {code} = experiments.swc.transformSync(
    `
      function Icon(props) {
        return (
          <svg viewBox="0 0 16 16" {...props}>
            <g><path d="M0 0L16 16" /></g>
            <Mark />
          </svg>
        );
      }

      function Chart({points}) {
        return <svg>{points.map((point) => <Dot key={point} />)}</svg>;
      }
    `,
    {
      filename: path.resolve("Icon.jsx"),
      jsc: {
        parser: {syntax: "ecmascript", jsx: true},
        transform: {react: {runtime: "automatic"}, reactCompiler: true},
        experimental: {plugins: [[pluginPath, sentryConfig]]},
      },
    }
  );

  assert.match(code, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Mark, \{\}\)/);
  assert.match(code, /function _temp\(point\) \{\s+return \/\*#__PURE__\*\/ _jsx\(Dot, \{\}, point\)/);
  assert.equal(code.match(/"data-sentry-source-file"/g)?.length, 2);
  assert.doesNotMatch(code, /"data-sentry-element"/);
});

test("fails the Rspack build on an invalid config", async t => {
  const {stats} = await build(t, {"transparent-components": "Flex"});

  assert.match(
    stats.toString({all: false, errors: true}),
    /swc-plugin-component-annotate: invalid config: invalid type: string "Flex", expected a sequence/
  );
});
