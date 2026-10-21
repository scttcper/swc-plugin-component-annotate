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

test("supports transparent memo owners in CommonJS scripts", () => {
  const code = transform(
    `
      const React = require("react");

      const MemoActions = React.memo(function MemoActions() {
        return <Grid />;
      });

      function Grid(props) {
        return <div {...props} />;
      }

      module.exports = MemoActions;
    `,
    {...sentryConfig, "transparent-components": ["Grid"]},
    {...reactCompiler, isModule: false}
  );

  assert.match(code, /"data-sentry-component": "MemoActions-Grid"/);
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

test("does not treat a later reassignment as a returned render root", () => {
  const code = transform(
    `
      function App({notify}) {
        let value = <Grid />;
        const root = value;
        value = <Button />;
        notify(value);
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
  assert.doesNotMatch(code, /"data-sentry-component": "App-Button"/);
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

test("attributes React Compiler cache values used by a logical return", () => {
  const code = transform(
    `
      function App({ready}) {
        const first = <Grid />;
        const root = ready && first;
        return root;
      }

      function Grid(props) {
        return <div {...props} />;
      }
    `,
    {...sentryConfig, "transparent-components": ["Grid"]},
    reactCompiler
  );

  assert.match(code, /"data-sentry-component": "App-Grid"/);
});

test("attributes separately cached JSX children", () => {
  const code = transform(
    `
      function App({label}) {
        const button = <Button />;
        return <Grid label={label}>{button}</Grid>;
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

test("does not annotate a Fragment hoisted into a JSX tag binding", () => {
  const code = transform(
    `
      import {Fragment} from "react";

      function App({items}) {
        const filtered = items.filter(Boolean);
        return <Fragment>{filtered.map(item => <Row item={item} />)}</Fragment>;
      }
    `,
    sentryConfig,
    reactCompiler
  );

  // This source currently makes React Compiler emit `T0 = Fragment` and then
  // use T0 as the JSX tag. The annotation pass runs against that output.
  assert.match(code, /T0 = Fragment/);
  assert.match(code, /_jsx\(T0, \{/);
  assert.doesNotMatch(code, /"data-sentry-element": "T0"/);
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

test("attributes JSX passed through calls in compiled components", () => {
  const code = transform(
    `
      import {createPortal} from "react-dom";

      function Notice() {
        return tct("Read our [link:privacy policy]", {link: <ExternalLink href="/privacy" />});
      }

      function Overlay({container}) {
        return createPortal(<div className="overlay" />, container);
      }

      function Summary({count, total}) {
        return tct("[count] of [total]", {
          count: <Count value={count} />,
          total: <Count value={total} />,
        });
      }
    `,
    sentryConfig,
    reactCompiler
  );

  assert.match(code, /_jsx\(ExternalLink, \{[^}]*"data-sentry-component": "Notice"/);
  assert.match(code, /_jsx\("div", \{[^}]*"data-sentry-component": "Overlay"/);
  assert.equal(code.match(/_jsx\(Count, \{[^}]*"data-sentry-component": "Summary"/g)?.length, 2);
});

test("gives transparent callsites in render callbacks their component as owner", () => {
  const code = transform(
    `
      function List({items, onSelect}) {
        return (
          <ul>
            {items.map((item) => <Button key={item.id}>{item.label}</Button>)}
            {items.map((item) => <Button key={item.id} onClick={() => onSelect(item)} />)}
          </ul>
        );
      }
    `,
    {...sentryConfig, "transparent-components": ["Button"]},
    reactCompiler
  );

  // The first callback is outlined into a `_temp` helper, the second stays inline
  // because it captures `onSelect`. Both render for List.
  assert.equal(code.match(/"data-sentry-component": "List-Button"/g)?.length, 2);
  assert.doesNotMatch(code, /"data-sentry-component": "_temp/);
});

test("only owns what render callbacks return", () => {
  const code = transform(
    `
      function App({items}) {
        return (
          <ul>
            {items.map((item) => {
              register(<Button id="registered" />);
              return <Button id="rendered" key={item.id} />;
            })}
          </ul>
        );
      }
    `,
    {...sentryConfig, "transparent-components": ["Button"]},
    reactCompiler
  );

  assert.match(code, /id: "rendered",\s+"data-sentry-component": "App-Button"/);
  assert.doesNotMatch(code, /id: "registered",\s+"data-sentry-component"/);
});

test("skips JSX the compiler hoists out of an svg", () => {
  const code = transform(
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
    sentryConfig,
    reactCompiler
  );

  // The svg children are cached ahead of it and the callback becomes a `_temp`
  // helper, so none of them are inside the svg by the time the plugin runs
  assert.match(code, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Mark, \{\}\)/);
  assert.match(code, /function _temp\(point\) \{\s+return \/\*#__PURE__\*\/ _jsx\(Dot, \{\}, point\)/);
  assert.equal(code.match(/"data-sentry-source-file"/g)?.length, 2);
  assert.match(code, /"data-sentry-component": "Icon"/);
  assert.match(code, /"data-sentry-component": "Chart"/);
  assert.doesNotMatch(code, /"data-sentry-element"/);
});

test("skips every branch the compiler picks for an svg", () => {
  const code = transform(
    `
      function Shape({filled, kind}) {
        let shape;
        if (filled) {
          shape = <Circle />;
        } else {
          shape = <Rect />;
        }

        let mark;
        switch (kind) {
          case "dot":
            mark = <Dot />;
            break;
          default:
            mark = <Cross />;
        }

        return <svg>{shape}{mark}</svg>;
      }
    `,
    sentryConfig,
    reactCompiler
  );

  // Each branch is cached on its own, then assigned to the binding the svg renders
  assert.equal(code.match(/shape = t\d+;/g)?.length, 2);
  assert.equal(code.match(/mark = t\d+;/g)?.length, 2);
  assert.match(code, /"data-sentry-component": "Shape"/);
  assert.equal(code.match(/"data-sentry-source-file"/g)?.length, 1);
  assert.doesNotMatch(code, /"data-sentry-element"/);
});

test("annotates JSX cached out of a returned element like an uncompiled child", () => {
  const source = `
    import {Fragment} from "react";

    function Parent({label}) {
      return (
        <div>
          <Child />
          <span className="label" />
          <Flex>{label}</Flex>
        </div>
      );
    }

    function Wrapper({label}) {
      return (
        <Fragment>
          {label}
          <Other />
        </Fragment>
      );
    }
  `;
  const config = {...sentryConfig, "transparent-components": ["Flex"]};

  for (const swcOptions of [{jsc: {transform: {react: {runtime: "automatic"}}}}, reactCompiler] as const) {
    const code = transform(source, config, swcOptions);

    // A returned element's children only get their own name, and transparent
    // callsites among them still take the component as their owner
    assert.match(code, /_jsx\(Child, \{\s+"data-sentry-element": "Child",\s+"data-sentry-source-file": "Button\.jsx"\s+\}\)/);
    assert.match(code, /_jsx\("span", \{\s+className: "label"\s+\}\)/);
    assert.match(code, /"data-sentry-component": "Parent-Flex"/);
    // A fragment's children render in its place, so they're roots
    assert.match(code, /_jsx\(Other, \{\s+"data-sentry-element": "Other",\s+"data-sentry-component": "Wrapper"/);
  }

  // The compiler caches the static children apart from what they're rendered in
  const compiled = transform(source, config, reactCompiler);
  assert.match(compiled, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Child, \{/);
  assert.match(compiled, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Other, \{/);
});

test("annotates JSX cached out of a returned element's props like its children", () => {
  const source = `
    import {Fragment} from "react";

    function Parent({label}) {
      return <Panel header={<Flex />}>{label}</Panel>;
    }

    function Hint({children}) {
      const title = (
        <Fragment>
          <Text />
          <Flex />
        </Fragment>
      );
      return <Tooltip title={title}>{children}</Tooltip>;
    }

    function Shared() {
      const icon = <Icon />;
      return (
        <>
          <Button icon={icon} />
          <svg>{icon}</svg>
        </>
      );
    }
  `;
  const config = {...sentryConfig, "transparent-components": ["Flex"]};

  for (const swcOptions of [{jsc: {transform: {react: {runtime: "automatic"}}}}, reactCompiler] as const) {
    const code = transform(source, config, swcOptions);

    assert.match(code, /_jsx\(Flex, \{\s+"data-sentry-component": "Parent-Flex"/);
    // Rendered through a prop as well as the svg, so it keeps its name
    assert.match(code, /_jsx\(Icon, \{\s+"data-sentry-element": "Icon",\s+"data-sentry-source-file": "Button\.jsx"\s+\}\)/);
    // A fragment inside a prop doesn't hand the component name to its children
    assert.match(code, /_jsx\(Text, \{\s+"data-sentry-element": "Text",\s+"data-sentry-source-file": "Button\.jsx"\s+\}\)/);
  }

  const compiled = transform(source, config, reactCompiler);
  assert.match(compiled, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Flex, \{/);
  assert.match(compiled, /t\d+ = \/\*#__PURE__\*\/ _jsx\(Icon, \{/);
  // The compiler traces the title into the prop, so Flex gets an owner here
  assert.match(compiled, /_jsx\(Flex, \{\s+"data-sentry-component": "Hint-Flex"/);
});
