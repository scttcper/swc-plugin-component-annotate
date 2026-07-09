import path from "node:path";
import {transformSync, type Options} from "@swc/core";

export type PluginConfig = Record<string, unknown>;

const pluginPath = path.resolve(
  "target/wasm32-unknown-unknown/release/swc_plugin_component_annotate.wasm"
);

/** Run source through @swc/core with the plugin wired in. */
export function transform(
  source: string,
  pluginConfig: PluginConfig = {},
  swcOptions: Pick<Options, "isModule" | "jsc"> = {}
): string {
  const jsc: NonNullable<Options["jsc"]> = {
    parser: {syntax: "ecmascript", jsx: true},
    experimental: {plugins: [[pluginPath, pluginConfig]]},
  };

  if (swcOptions.jsc?.transform) {
    jsc.transform = swcOptions.jsc.transform;
  }

  const options: Options = {
    filename: path.resolve("tests/e2e/Button.jsx"),
    jsc,
  };
  if (swcOptions.isModule !== undefined) {
    options.isModule = swcOptions.isModule;
  }

  return transformSync(source, options).code;
}

/** SWC options that enable the automatic runtime + React Compiler. */
export const reactCompiler: Pick<Options, "jsc"> = {
  jsc: {transform: {react: {runtime: "automatic"}, reactCompiler: true}},
};

/** Attribute names used by the Sentry-style config across tests. */
export const sentryConfig: PluginConfig = {
  "component-attr": "data-sentry-component",
  "element-attr": "data-sentry-element",
  "source-file-attr": "data-sentry-source-file",
};
