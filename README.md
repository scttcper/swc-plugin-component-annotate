# SWC Plugin: Component Annotate [![npm badge](https://img.shields.io/npm/v/swc-plugin-component-annotate)](https://www.npmjs.com/package/swc-plugin-component-annotate)

A SWC plugin that automatically annotates React components with data attributes for component tracking and debugging.

## Overview

This plugin transforms React components by adding data attributes that help with tracking and debugging. It automatically identifies React components (function components, arrow function components, and class components) and adds the following attributes:

- `data-component`: The component name (added to root elements)
- `data-element`: The element/component name (added to non-HTML elements)
- `data-source-file`: The source filename

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

- **`transparent-components`** (array, default: `[]`): List of pass-through component names that should keep the nearest owning component annotation instead of being reported as their own element. This is useful for layout primitives such as `Flex`, `Stack`, `Grid`, or `Container`, and for design-system primitives when owner-only DOM annotations are preferred.

- **`component-attr`** (string, optional): Custom component attribute name (overrides default and native setting)

- **`element-attr`** (string, optional): Custom element attribute name (overrides default and native setting)

- **`source-file-attr`** (string, optional): Custom source file attribute name (overrides default and native setting)

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

In this example, `Flex` is layout plumbing. With `Flex` configured as transparent, the layout wrapper keeps the owning component metadata without reporting itself as the element:

```jsx
function IssueActions() {
  return (
    <Flex data-sentry-component="IssueActions" data-sentry-source-file="issueActions.tsx">
      <Button data-sentry-element="Button" data-sentry-source-file="issueActions.tsx">
        Resolve
      </Button>
    </Flex>
  );
}
```

If the React component `Button` is also configured as transparent, its annotation intentionally collapses to the owner:

```jsx
function IssueActions() {
  return (
    <Flex data-sentry-component="IssueActions" data-sentry-source-file="issueActions.tsx">
      <Button data-sentry-component="IssueActions" data-sentry-source-file="issueActions.tsx">
        Resolve
      </Button>
    </Flex>
  );
}
```

For a `Button` implementation that forwards props to a `button`, those owner attributes become the final DOM attributes.

If a transparent component renders a polymorphic DOM element internally, that implementation is not annotated. Forwarded attributes can pass through instead of being overwritten by the transparent component's own file:

```jsx
const Container = memo(function Container({as: Component = 'div', ...props}) {
  return <Component {...props} />;
});
```

The useful result is that layout DOM points at the owner that rendered it, not at `Flex` or `flex.tsx`:

```html
<div data-sentry-component="IssueActions" data-sentry-source-file="issueActions.tsx">
  ...
</div>
```

Whether to include components such as `Button`, `Link`, `Checkbox`, or `MenuItem` depends on the signal you want in the DOM. Leave them out of `transparent-components` when their component name is useful context, for example "the `Button` rendered by `IssueActions`". Include them when the owning component is the preferred label everywhere, for example "the `IssueActions` UI rendered this DOM node".

Use `ignored-components` only when a component should not participate in annotation at all. Ignored components get no generated attributes. Use `transparent-components` when a component should still receive useful owner metadata, but should not report itself as `data-sentry-element`.

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
