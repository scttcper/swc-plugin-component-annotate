import assert from "node:assert/strict";
import path from "node:path";
import {test} from "node:test";
import {transformSync, type Options} from "@swc/core";

type PluginConfig = Record<string, unknown>;

const pluginPath = path.resolve(
  "target/wasm32-unknown-unknown/release/swc_plugin_component_annotate.wasm"
);

function transform(source: string, pluginConfig: PluginConfig = {}): string {
  const options: Options = {
    filename: path.resolve("tests/e2e/Button.jsx"),
    jsc: {
      parser: {
        syntax: "ecmascript",
        jsx: true,
      },
      experimental: {
        plugins: [[pluginPath, pluginConfig]],
      },
    },
  };

  return transformSync(source, options).code;
}

test("annotates React components through @swc/core", () => {
  const code = transform(`
    function Button() {
      return <div><Icon /></div>;
    }
  `);

  assert.match(code, /"data-component": "Button"/);
  assert.match(code, /"data-source-file": "Button\.jsx"/);
  assert.match(code, /React\.createElement\(Icon, \{\s+"data-element": "Icon"/);
});

test("passes plugin config through the SWC wasm boundary", () => {
  const code = transform(
    `
      function Button() {
        return <Icon />;
      }
    `,
    {
      "component-attr": "data-sentry-component",
      "element-attr": "data-sentry-element",
      "source-file-attr": "data-sentry-source-file",
    }
  );

  assert.match(code, /"data-sentry-component": "Button"/);
  assert.match(code, /"data-sentry-element": "Icon"/);
  assert.match(code, /"data-sentry-source-file": "Button\.jsx"/);
});

test("passes owner metadata through transparent component callsites", () => {
  const code = transform(
    `
      function MergedItem() {
        return (
          <MergedGroup>
            <Grid>
              <Flex>
                <Button aria-label="Show fingerprints" />
              </Flex>
            </Grid>
          </MergedGroup>
        );
      }

      function MergedGroup(props) {
        return <div {...props} />;
      }

      function Grid(props) {
        return <div {...props} />;
      }

      function Flex(props) {
        return <div {...props} />;
      }

      function Button(props) {
        return <button {...props} />;
      }
    `,
    {
      "component-attr": "data-sentry-component",
      "element-attr": "data-sentry-element",
      "source-file-attr": "data-sentry-source-file",
      "transparent-components": ["Button", "Flex", "Grid"],
    }
  );

  assert.match(code, /React\.createElement\(Button, \{\s+"aria-label": "Show fingerprints",\s+"data-sentry-component": "MergedItem"/);
  assert.doesNotMatch(code, /"data-sentry-element": "Button"/);
  assert.match(code, /React\.createElement\("button", props\)/);
});
