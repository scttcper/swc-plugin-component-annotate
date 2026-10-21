# SWC Plugin: Component Annotate [![npm badge](https://img.shields.io/npm/v/swc-plugin-component-annotate)](https://www.npmjs.com/package/swc-plugin-component-annotate)

A SWC plugin that automatically annotates React components with data attributes for component tracking and debugging.

## Overview

This plugin transforms React components by adding data attributes that help with tracking and debugging. It automatically identifies React components (function components, arrow function components, and class components) and adds the following attributes:

- `data-component`: The component name (added to root elements)
- `data-element`: The name of a rendered component, like `Button` (left off tags like `div`)
- `data-source-file`: The source filename (left off when the host passes none)

## Features

- ✅ **Function Components**: `function MyComponent() { ... }`
- ✅ **Arrow Function Components**: `const MyComponent = () => { ... }`
- ✅ **Class Components**: `class MyComponent extends Component { ... }`
- ✅ **React Fragments**: Supports `Fragment`, `React.Fragment`, and `<>` syntax
- ✅ **Nested Components**: Properly handles component hierarchies
- ✅ **React Native Support**: Uses camelCase attributes when configured
- ✅ **Configurable**: Ignore specific components, annotate fragments, etc.

## Installation

```bash
npm install --save-dev swc-plugin-component-annotate
```

### Compatibility

Version 1.18.x is compiled against `swc_core` 77.0.0 and tested with
`@swc/core` 1.16.1 through 1.16.18 and `@rspack/core` 2.2.0 through
2.3.0-beta.0. The wasm ABI comes from `swc_ecma_ast` 29 and `swc_common` 26,
which every `swc_core` release from 77.0.0 through 82.1.1 shares. You only
need a newer plugin release once your host moves to a new major of either
crate.

## Usage

### Basic Configuration

Add the plugin to your `.swcrc` configuration:

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        ["swc-plugin-component-annotate", {}]
      ]
    }
  }
}
```

### Configuration Options

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        ["swc-plugin-component-annotate", {
          "native": false,
          "ignored-components": ["MyIgnoredComponent"],
          "transparent-components": ["Flex", "Stack"],
          "component-attr": "data-sentry-component",
          "element-attr": "data-sentry-element",
          "source-file-attr": "data-sentry-source-file"
        }]
      ]
    }
  }
}
```

#### Options

- **`native`** (boolean, default: `false`): Use React Native attribute names (camelCase)
  - `false`: `data-component`, `data-element`, `data-source-file`
  - `true`: `dataComponent`, `dataElement`, `dataSourceFile`

- **`ignored-components`** (array, default: `[]`): List of component names to skip during annotation

- **`transparent-components`** (array, default: `[]`): List of pass-through component names that should receive an `Owner-Component` annotation instead of being reported as their own element. This is useful for layout primitives such as `Flex`, `Stack`, `Grid`, or `Container`, and for design-system primitives when caller context is preferred.

- **`component-attr`** (string, optional): Custom component attribute name (overrides default and native setting)

- **`element-attr`** (string, optional): Custom element attribute name (overrides default and native setting)

- **`source-file-attr`** (string, optional): Custom source file attribute name (overrides default and native setting)

- **`source-path-attr`** (string, optional): Adds the file's full path under this attribute name. It's only emitted when set, so leave it out of production builds to keep bundles small.

- **`rewrite-emotion-styled`** (boolean, default: `false`, experimental): Rewrites `const StyledButton = styled(Button)` (with `styled` from `@emotion/styled`) so the rendered `Button` gets `data-element="StyledButton"` plus the source attributes.

An option with the wrong type, like `"transparent-components": "Flex"`, fails the build with `swc-plugin-component-annotate: invalid config: ...` rather than falling back to the defaults.

### Sentry Integration

To use Sentry-specific attribute names for compatibility with Sentry's tracking:

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        ["swc-plugin-component-annotate", {
          "component-attr": "data-sentry-component",
          "element-attr": "data-sentry-element",
          "source-file-attr": "data-sentry-source-file",
          "transparent-components": ["Container", "Flex", "Grid", "Stack"]
        }]
      ]
    }
  }
}
```

This will generate attributes like:
```jsx
<div data-sentry-component="MyComponent" data-sentry-source-file="MyComponent.jsx">
  <CustomButton data-sentry-element="CustomButton" data-sentry-source-file="MyComponent.jsx">
    Click me
  </CustomButton>
</div>
```

#### Transparent Components

Use `transparent-components` for components where the caller is more useful context than the component's own implementation. Layout primitives are the typical case:

```jsx
function IssueActions() {
  return (
    <Flex>
      <Button>Resolve</Button>
    </Flex>
  );
}
```

In this example, `Flex` is layout plumbing. With `Flex` configured as transparent, the layout wrapper records both its owner and its own callsite name without reporting itself as the element:

```jsx
function IssueActions() {
  return (
    <Flex data-sentry-component="IssueActions-Flex" data-sentry-source-file="issueActions.tsx">
      <Button data-sentry-element="Button" data-sentry-source-file="issueActions.tsx">
        Resolve
      </Button>
    </Flex>
  );
}
```

Those are the callsite attributes. A component that spreads its props onto a DOM element is annotated too, and its own attributes go after the spread, so they replace the forwarded ones. If `Button` renders `<button {...props}>`, the DOM ends up with `Button`'s component name and file:

```html
<button data-sentry-element="Button" data-sentry-component="Button" data-sentry-source-file="button.tsx">
  Resolve
</button>
```

Sentry's SDK names a clicked node by walking up at most five levels and taking the first `data-sentry-component`, then `data-sentry-element`, so every click on this button reports `Button` from `button.tsx`.

If the React component `Button` is also configured as transparent, it gets the same root owner with its own component name. Transparent wrappers do not accumulate in nested labels:

```jsx
function IssueActions() {
  return (
    <Flex data-sentry-component="IssueActions-Flex" data-sentry-source-file="issueActions.tsx">
      <Button data-sentry-component="IssueActions-Button" data-sentry-source-file="issueActions.tsx">
        Resolve
      </Button>
    </Flex>
  );
}
```

For a `Button` implementation that forwards props to a `button`, those owner attributes become the final DOM attributes.

Transparent callsites inside render callbacks, such as `items.map((item) => <Button />)` in the returned JSX, take the rendering component as their owner too. JSX the component doesn't render, like an argument to `register()`, gets none.

If a transparent component renders a polymorphic DOM element internally, that implementation is not annotated. Forwarded attributes can pass through instead of being overwritten by the transparent component's own file:

```jsx
const Container = memo(function Container({as: Component = 'div', ...props}) {
  return <Component {...props} />;
});
```

The useful result is that layout DOM retains both the owner and the transparent callsite, rather than pointing at `Flex` or `flex.tsx` alone:

```html
<div data-sentry-component="IssueActions-Flex" data-sentry-source-file="issueActions.tsx">
  ...
</div>
```

Whether to include components such as `Button`, `Link`, `Checkbox`, or `MenuItem` depends on the signal you want in the DOM. Leave them out of `transparent-components` when reporting the component itself, `Button` from `button.tsx`, is enough. Include them when the caller matters, so clicks report labels like `IssueActions-Button` with the caller's file.

Use `ignored-components` only when a component should not participate in annotation at all. Ignored components get no generated attributes. Use `transparent-components` when a component should still receive useful owner metadata, but should not report itself as `data-sentry-element`.

#### What's left out

Names that wouldn't tell you what rendered are dropped rather than replaced, so the nearest real component gets reported instead:

- Only PascalCase functions count as components. Helpers that return JSX, like `renderRow` or `getIcon`, don't add `data-sentry-component`.
- Lowercase tags, like `<div>`, `<path>` or `<my-element>`, don't get `data-sentry-element`. The DOM already shows the tag.
- Polymorphic `<Component>` tags and members of local values, like `<field.Switch>`, don't get `data-sentry-element`. Members of component namespaces, like `<Fields.Switch>`, keep theirs.
- Nothing inside an `<svg>` gets attributes, including components and callbacks rendered there. Paths and shapes don't say what was clicked. The `<svg>` itself is annotated like any other tag.

## Examples

### Input

```jsx
import React from 'react';

const MyComponent = () => {
  return (
    <div>
      <h1>Hello World</h1>
      <button>Click me</button>
    </div>
  );
};

export default MyComponent;
```

### Output

```jsx
import React from 'react';

const MyComponent = () => {
  return (
    <div data-component="MyComponent" data-source-file="MyComponent.jsx">
      <h1>Hello World</h1>
      <button>Click me</button>
    </div>
  );
};

export default MyComponent;
```

### Class Component Example

```jsx
// Input
class MyClassComponent extends Component {
  render() {
    return <div><h1>Hello from class</h1></div>;
  }
}

// Output
class MyClassComponent extends Component {
  render() {
    return <div data-component="MyClassComponent" data-source-file="MyComponent.jsx">
      <h1>Hello from class</h1>
    </div>;
  }
}
```

### React Native Example

With `"native": true`:

```jsx
// Output
const MyComponent = () => {
  return (
    <View dataComponent="MyComponent" dataSourceFile="MyComponent.jsx">
      <Text>Hello World</Text>
    </View>
  );
};
```

## Related

- [Sentry Babel Component Annotate Plugin](https://github.com/getsentry/sentry-javascript-bundler-plugins/tree/main/packages/babel-plugin-component-annotate)
