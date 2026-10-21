import assert from "node:assert/strict";
import {test} from "node:test";

import {transformSync} from "@swc/core";

import {pluginPath, sentryConfig, transform, typescript} from "./helpers.ts";

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
    sentryConfig
  );

  assert.match(code, /"data-sentry-component": "Button"/);
  assert.match(code, /"data-sentry-element": "Icon"/);
  assert.match(code, /"data-sentry-source-file": "Button\.jsx"/);
});

test("leaves out source attributes when there's no filename", () => {
  const {code} = transformSync(`function Button() { return <div><Icon /></div>; }`, {
    jsc: {
      parser: {syntax: "ecmascript", jsx: true},
      experimental: {
        plugins: [[pluginPath, {...sentryConfig, "source-path-attr": "data-sentry-source-path"}]],
      },
    },
  });

  assert.match(code, /"data-sentry-component": "Button"/);
  assert.match(code, /"data-sentry-element": "Icon"/);
  assert.doesNotMatch(code, /data-sentry-source-(file|path)/);
});

test("fails on an invalid config instead of falling back to the defaults", () => {
  assert.throws(
    () => transform(`function Button() { return <button />; }`, {"transparent-components": "Flex"}),
    /swc-plugin-component-annotate: invalid config: invalid type: string "Flex", expected a sequence/
  );
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
    {...sentryConfig, "transparent-components": ["Button", "Flex", "Grid"]}
  );

  assert.match(code, /React\.createElement\(Grid, \{\s+"data-sentry-component": "MergedItem-Grid"/);
  assert.match(code, /React\.createElement\(Flex, \{\s+"data-sentry-component": "MergedItem-Flex"/);
  assert.match(code, /React\.createElement\(Button, \{\s+"aria-label": "Show fingerprints",\s+"data-sentry-component": "MergedItem-Button"/);
  assert.doesNotMatch(code, /"data-sentry-element": "Button"/);
  assert.match(code, /React\.createElement\("button", props\)/);
});

test("annotates TypeScript components through @swc/core", () => {
  const code = transform(
    `
      import {memo, type FC} from "react";

      type Props = {label: string};

      export const Typed: FC<Props> = ({label}) => <span>{label}</span>;
      export const Cast = (({label}: Props) => <span>{label}</span>) as FC<Props>;
      export const Satisfies = (({label}: Props) => <span>{label}</span>) satisfies FC<Props>;
      export const Memo = memo(({label}: Props) => <span>{label}</span>) as typeof Typed;
    `,
    sentryConfig,
    typescript
  );

  for (const name of ["Typed", "Cast", "Satisfies", "Memo"]) {
    assert.match(code, new RegExp(`"data-sentry-component": "${name}"`));
  }
  assert.match(code, /"data-sentry-source-file": "Button\.tsx"/);
});
