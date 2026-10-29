import React from 'react';
export default function MyDefaultComponent({ title }) {
    return <Layout data-element="Layout" data-component="MyDefaultComponent" data-source-file="test.jsx">
      <h1>{title}</h1>
      <Button data-element="Button" data-source-file="test.jsx">Click me</Button>
    </Layout>;
}
