# SWC Plugin: Component Annotate

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
# or
yarn add -D swc-plugin-component-annotate
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
          "annotate-fragments": false,
          "ignored-components": ["MyIgnoredComponent"],
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

- **`annotate-fragments`** (boolean, default: `false`): Whether to annotate fragment children with component information

- **`ignored-components`** (array, default: `[]`): List of component names to skip during annotation

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
          "source-file-attr": "data-sentry-source-file"
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