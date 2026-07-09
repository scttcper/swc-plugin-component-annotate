import assert from "node:assert/strict";
import {test} from "node:test";

import {reactCompiler, sentryConfig, transform} from "./helpers.ts";

// The React Compiler runs before this plugin and hoists returned JSX out of
// `return` into memo-cache assignments (`t = <jsx>`). These tests cover that
// the owner still reaches those hoisted roots.

test("passes owner metadata through transparent component callsites with React Compiler enabled", () => {
  const code = transform(
    `
      function MergedItem() {
        return (
          <Grid>
            <Button aria-label="Show fingerprints" />
          </Grid>
        );
      }

      function Grid(props) {
        return <div {...props} />;
      }

      function Button(props) {
        return <button {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Button", "Grid"]},
    reactCompiler
  );

  assert.match(code, /_jsx\(Grid, \{\s+"data-sentry-component": "MergedItem-Grid"/);
  assert.match(code, /_jsx\(Button, \{\s+"aria-label": "Show fingerprints",\s+"data-sentry-component": "MergedItem-Button"/);
  assert.doesNotMatch(code, /"data-sentry-element": "Button"/);
  assert.match(code, /_jsx\("button", _object_spread\(\{\}, props\)\)/);
});

test("attributes transparent callsites owned by memo components", () => {
  const source = `
    import React, {memo} from "react";

    const MemoActions = memo(function MemoActions() {
      return <Button />;
    });

    const NamespaceMemoActions = React.memo(function NamespaceMemoActions() {
      return <Button />;
    });

    function Button(props) {
      return <button {...props} />;
    }
  `;

  for (const [runtime, swcOptions] of [
    ["classic", {}],
    ["react compiler", reactCompiler],
  ] as const) {
    const code = transform(
      source,
      {...sentryConfig, "transparent-components": ["Button"]},
      swcOptions
    );

    assert.match(
      code,
      /"data-sentry-component": "MemoActions-Button"/,
      `${runtime} should keep the memo component as the transparent owner`
    );
    assert.match(
      code,
      /"data-sentry-component": "NamespaceMemoActions-Button"/,
      `${runtime} should keep the React.memo component as the transparent owner`
    );
  }
});

test("does not treat non-return JSX as a React Compiler render root", () => {
  const code = transform(
    `
      function App() {
        register(<Grid />);
        return <div />;
      }

      function Grid(props) {
        return <div {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Grid"]},
    reactCompiler
  );

  assert.match(code, /register\([\s\S]*?_jsx\(Grid,/);
  assert.doesNotMatch(code, /"data-sentry-component": "App-Grid"/);
});

test("attributes React Compiler cache values forwarded through a returned local", () => {
  const code = transform(
    `
      function App({ready}) {
        let root;
        if (ready) {
          root = (
            <Grid>
              <Button />
            </Grid>
          );
        }
        return root;
      }

      function Grid(props) {
        return <div {...props} />;
      }

      function Button(props) {
        return <button {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Button", "Grid"]},
    reactCompiler
  );

  assert.match(code, /"data-sentry-component": "App-Grid"/);
  assert.match(code, /"data-sentry-component": "App-Button"/);
});

test("attributes React Compiler cache values selected by a returned conditional", () => {
  const code = transform(
    `
      function App({ready}) {
        let root;
        const first = <Grid />;
        const second = <Button />;
        root = ready ? first : second;
        return root;
      }

      function Grid(props) {
        return <div {...props} />;
      }

      function Button(props) {
        return <button {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Button", "Grid"]},
    reactCompiler
  );

  assert.match(code, /"data-sentry-component": "App-Grid"/);
  assert.match(code, /"data-sentry-component": "App-Button"/);
});

test("annotates cached React Compiler return values", () => {
  const code = transform(
    `
      function Button() {
        return <button />;
      }
    `,
    {},
    reactCompiler
  );

  assert.match(code, /_jsx\("button", \{\s+"data-component": "Button"/);
});

test("annotates nested cached React Compiler return values", () => {
  const code = transform(
    `
      function ConditionalActions({enabled}) {
        if (enabled) {
          return <Button />;
        }

        return <div />;
      }

      function Button(props) {
        return <button {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Button"]},
    reactCompiler
  );

  assert.match(code, /_jsx\(Button, \{\s+"data-sentry-component": "ConditionalActions-Button"/);
  assert.match(code, /_jsx\("div", \{\s+"data-sentry-component": "ConditionalActions"/);
  assert.doesNotMatch(code, /"data-sentry-element": "Button"/);
});

test("does not attribute React Compiler temp helpers as component owners", () => {
  const code = transform(
    `
      function List({items}) {
        return <ul>{items.map((i) => <li>{i}</li>)}</ul>;
      }
    `,
    sentryConfig,
    reactCompiler
  );

  assert.match(code, /"data-sentry-component": "List"/);
  assert.doesNotMatch(code, /"data-sentry-component": "_temp"/);
});

test("still annotates elements inside React Compiler temp helpers without an owner", () => {
  const code = transform(
    `
      function List({items}) {
        return <ul>{items.map((i) => <Row />)}</ul>;
      }
    `,
    sentryConfig,
    reactCompiler
  );

  assert.doesNotMatch(code, /"data-sentry-component": "_temp"/);
  assert.match(code, /_jsx\(Row, \{\s+"data-sentry-element": "Row"/);
});

// Rename-proof guard for the `is_react_compiler_temp` heuristic: instead of
// matching the literal `_temp` name, assert the real invariant — only
// source-defined components may become owners. A leak under any generated name
// fails here, and the failure lists the offending name.
test("only source-defined components become owners (guards against compiler helper renames)", () => {
  const code = transform(
    `
      function List({items}) {
        return (
          <ul onClick={() => <b />}>
            {items.map((i) => <li>{i}</li>)}
          </ul>
        );
      }
    `,
    sentryConfig,
    reactCompiler
  );

  const owners = new Set(
    [...code.matchAll(/"data-sentry-component": "([^"]+)"/g)].map((match) => match[1])
  );
  assert.deepEqual([...owners], ["List"]);
});
